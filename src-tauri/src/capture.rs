use base64::Engine;
use image::{imageops, RgbaImage};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution};
use nokhwa::Camera;

/// 截取指定显示器的画面，缩放到 max_width 以内。
pub fn capture_screen(monitor_index: usize, max_width: u32) -> Result<RgbaImage, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("获取显示器列表失败: {e}"))?;
    if monitors.is_empty() {
        return Err("未检测到显示器（Linux 下需要 X11 会话，Wayland 暂不支持屏幕捕获）。".into());
    }
    let idx = monitor_index.min(monitors.len() - 1);
    let img = monitors[idx]
        .capture_image()
        .map_err(|e| format!("屏幕捕获失败（macOS 请先在系统设置中授予「屏幕录制」权限）: {e}"))?;
    Ok(downscale(img, max_width))
}

/// 从指定摄像头抓一帧，缩放到 max_width 以内。
pub fn capture_camera(camera_index: usize, max_width: u32) -> Result<RgbaImage, String> {
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::HighestResolution(
        Resolution::new(1280, 720),
    ));
    let mut camera = Camera::new(CameraIndex::Index(camera_index as u32), requested)
        .map_err(|e| format!("打开摄像头失败（请检查系统相机权限/设备占用）: {e}"))?;
    let buffer = camera
        .frame()
        .map_err(|e| format!("读取摄像头帧失败: {e}"))?;
    let w = buffer.resolution().width();
    let h = buffer.resolution().height();
    if w == 0 || h == 0 {
        return Err("摄像头返回了空分辨率".into());
    }
    let bytes = buffer.buffer();
    let img = match buffer.source_frame_format() {
        FrameFormat::RAWRGB => rgb_to_rgba(w, h, bytes),
        FrameFormat::RAWBGR => bgr_to_rgba(w, h, bytes),
        FrameFormat::MJPEG => image::load_from_memory(bytes)
            .map(|i| i.to_rgba8())
            .map_err(|e| format!("解码摄像头 MJPEG 帧失败: {e}")),
        FrameFormat::YUYV => Ok(yuyv_to_rgba(w, h, bytes)),
        other => Err(format!("不支持的摄像头像素格式: {other:?}")),
    }?;
    Ok(downscale(img, max_width))
}

/// 编码为 JPEG 的 base64，用于发送给 Ollama 视觉模型。
pub fn to_jpeg_base64(img: &RgbaImage, quality: u8) -> Result<String, String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .encode_image(img)
        .map_err(|e| format!("JPEG 编码失败: {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
}

pub fn list_monitors() -> Result<Vec<String>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| format!("获取显示器失败: {e}"))?;
    Ok(monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m.name().unwrap_or_else(|_| "未知".into());
            format!("#{} {}  {}x{}", i, name, m.width().unwrap_or(0), m.height().unwrap_or(0))
        })
        .collect())
}

pub fn list_cameras() -> Result<Vec<String>, String> {
    let backend = nokhwa::native_api_backend().ok_or("当前平台无可用摄像头后端")?;
    let cameras = nokhwa::query(backend).map_err(|e| format!("枚举摄像头失败: {e}"))?;
    Ok(cameras
        .iter()
        .enumerate()
        .map(|(i, c)| format!("#{} {}", i, c.human_name()))
        .collect())
}

fn downscale(img: RgbaImage, max_width: u32) -> RgbaImage {
    if max_width == 0 || img.width() <= max_width {
        return img;
    }
    let h = ((img.height() as u64 * max_width as u64) / img.width() as u64) as u32;
    imageops::thumbnail(&img, max_width, h.max(1))
}

fn rgb_to_rgba(w: u32, h: u32, bytes: &[u8]) -> Result<RgbaImage, String> {
    let need = w as usize * h as usize * 3;
    if bytes.len() < need {
        return Err("摄像头 RGB 数据长度不足".into());
    }
    let mut data = Vec::with_capacity(w as usize * h as usize * 4);
    for chunk in bytes[..need].chunks_exact(3) {
        data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    RgbaImage::from_raw(w, h, data).ok_or_else(|| "RGBA 数据构造失败".into())
}

fn bgr_to_rgba(w: u32, h: u32, bytes: &[u8]) -> Result<RgbaImage, String> {
    let need = w as usize * h as usize * 3;
    if bytes.len() < need {
        return Err("摄像头 BGR 数据长度不足".into());
    }
    let mut data = Vec::with_capacity(w as usize * h as usize * 4);
    for chunk in bytes[..need].chunks_exact(3) {
        data.extend_from_slice(&[chunk[2], chunk[1], chunk[0], 255]);
    }
    RgbaImage::from_raw(w, h, data).ok_or_else(|| "RGBA 数据构造失败".into())
}

fn yuyv_to_rgba(w: u32, h: u32, bytes: &[u8]) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    let total = w as usize * h as usize;
    for i in 0..total {
        let group = i / 2;
        let y = bytes[i * 2] as f32;
        let u = bytes[group * 4 + 1] as f32 - 128.0;
        let v = bytes[group * 4 + 3] as f32 - 128.0;
        let r = (y + 1.402 * v).clamp(0.0, 255.0);
        let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0);
        let b = (y + 1.772 * u).clamp(0.0, 255.0);
        out.put_pixel(
            (i % w as usize) as u32,
            (i / w as usize) as u32,
            image::Rgba([r as u8, g as u8, b as u8, 255]),
        );
    }
    out
}
