// 生成 1024x1024 RGBA PNG（纯 Node，无第三方依赖），用作应用图标源文件。
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const W = 1024;
const H = 1024;
const buf = Buffer.alloc(W * H * 4);
const cx = W / 2;
const cy = H / 2;

function set(x, y, r, g, b, a) {
  const i = (y * W + x) * 4;
  buf[i] = r;
  buf[i + 1] = g;
  buf[i + 2] = b;
  buf[i + 3] = a;
}

for (let y = 0; y < H; y++) {
  for (let x = 0; x < W; x++) {
    const dx = x - cx;
    const dy = y - cy;
    const d = Math.sqrt(dx * dx + dy * dy);
    let r = 15, g = 18, b = 32, a = 255; // 深蓝底
    if (d > 300 && d < 345) {
      r = 79; g = 124; b = 255; // 外环（聚焦环）
    } else if (d < 225) {
      r = 52; g = 211; b = 153; // 中心绿圆
    }
    if (d < 95) {
      r = 15; g = 18; b = 32; // 中心孔
    }
    set(x, y, r, g, b, a);
  }
}

function crc32(bufIn) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = [];
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c >>> 0;
    }
  }
  let c = 0xffffffff;
  for (let i = 0; i < bufIn.length; i++)
    c = table[(c ^ bufIn[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

const raw = Buffer.alloc((W * 4 + 1) * H);
for (let y = 0; y < H; y++) {
  raw[y * (W * 4 + 1)] = 0; // filter: None
  buf.copy(raw, y * (W * 4 + 1) + 1, y * W * 4, (y + 1) * W * 4);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync("app-icon.png", png);
console.log("app-icon.png written:", png.length, "bytes");
