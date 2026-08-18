#!/usr/bin/env python3
import json
import struct
import sys
from pathlib import Path


path = Path(sys.argv[1])
raw = path.read_bytes()
if raw[:4] == bytes.fromhex("28b52ffd"):
    decoded = Path("/tmp/arch_state-v3.bin").read_bytes()
else:
    decoded = raw
    decoded = raw
magic, version, total, info_len = struct.unpack_from("<4I", decoded, 0)
info = json.loads(decoded[16:16 + info_len])
state = info["state"]
infos = info["buffer_infos"]
print(f"magic=0x{magic:08x} version={version} total={total} info_len={info_len}")
print(f"state_len={len(state)} buffers={len(infos)} decoded={len(decoded)}")
for index, value in enumerate(state):
    if isinstance(value, dict) and "buffer_id" in value:
        bid = value["buffer_id"]
        print(f"state[{index}] typed_buffer id={bid} len={infos[bid]['length']}")
    elif isinstance(value, list):
        print(f"state[{index}] array len={len(value)}")
    elif value is not None:
        print(f"state[{index}] scalar={value}")
for index, item in enumerate(infos):
    print(f"buffer[{index}] offset={item['offset']} length={item['length']}")
