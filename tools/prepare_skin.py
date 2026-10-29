"""Verify the generated RGBA buffer against the supplied 64x64 skin PNG."""
import argparse
from pathlib import Path
import struct
import zlib


def decode(path):
    data = path.read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('Not a PNG')
    offset, compressed, header = 8, bytearray(), None
    while offset < len(data):
        size = struct.unpack_from('>I', data, offset)[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + size]
        crc = struct.unpack_from('>I', data, offset + 8 + size)[0]
        if zlib.crc32(kind + payload) & 0xffffffff != crc:
            raise ValueError('PNG CRC mismatch')
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', payload)
        elif kind == b'IDAT':
            compressed.extend(payload)
        offset += size + 12
    if header != (64, 64, 8, 6, 0, 0, 0):
        raise ValueError('Expected a noninterlaced 64x64 RGBA8 skin')
    raw = zlib.decompress(compressed)
    stride = 64 * 4
    if len(raw) != (stride + 1) * 64:
        raise ValueError('Invalid PNG scanline length')
    output, previous = bytearray(), bytearray(stride)
    for row in range(64):
        mode = raw[row * (stride + 1)]
        line = bytearray(raw[row * (stride + 1) + 1:(row + 1) * (stride + 1)])
        for x in range(stride):
            left = line[x - 4] if x >= 4 else 0
            above = previous[x]
            corner = previous[x - 4] if x >= 4 else 0
            p = left + above - corner
            distances = [abs(p - left), abs(p - above), abs(p - corner)]
            paeth = [left, above, corner][distances.index(min(distances))]
            if mode not in range(5):
                raise ValueError('Unknown PNG filter')
            line[x] = (line[x] + [0, left, above, (left + above) // 2, paeth][mode]) & 255
        output.extend(line)
        previous = line
    return bytes(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    assets = Path(__file__).resolve().parents[1] / 'assets'
    pixels = decode(assets / 'herobrine.png')
    target = assets / 'herobrine.rgba'
    if args.write:
        target.write_bytes(pixels)
    elif target.read_bytes() != pixels:
        raise SystemExit('Skin buffer is stale: python tools/prepare_skin.py --write')
    print('Skin PNG and embedded RGBA pixels match')
