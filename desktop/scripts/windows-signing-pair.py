#!/usr/bin/env python3
"""Check that adding an Authenticode table did not replace the unsigned PE payload.

This verifies byte preservation only. Windows must independently verify the
signature, expected certificate, timestamp and trust before release admission.
Format: https://learn.microsoft.com/en-us/windows/win32/debug/pe-format
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import struct


MAX_BYTES = 512 * 1024 * 1024


class SigningPairError(ValueError):
    pass


def pe_fields(body: bytes) -> tuple[int, int]:
    if len(body) < 64 or body[:2] != b'MZ':
        raise SigningPairError('Expected a PE image')
    pe = struct.unpack_from('<I', body, 60)[0]
    if pe < 64 or pe + 24 > len(body) or body[pe:pe + 4] != b'PE\0\0':
        raise SigningPairError('Invalid PE header')
    optional = pe + 24
    length = struct.unpack_from('<H', body, pe + 20)[0]
    if length < 2 or optional + length > len(body):
        raise SigningPairError('Truncated optional header')
    magic = struct.unpack_from('<H', body, optional)[0]
    directory_start = {0x10b: 96, 0x20b: 112}.get(magic)
    if directory_start is None or length < directory_start + 40:
        raise SigningPairError('Unsupported PE optional header')
    count = struct.unpack_from('<I', body, optional + directory_start - 4)[0]
    if count < 5 or count > (length - directory_start) // 8:
        raise SigningPairError('Invalid data directory count')
    return optional + 64, optional + directory_start + 32


def verify_bytes(unsigned: bytes, signed: bytes) -> dict:
    if max(len(unsigned), len(signed)) > MAX_BYTES:
        raise SigningPairError('PE image exceeds the inspection limit')
    checksum, directory = pe_fields(unsigned)
    if pe_fields(signed) != (checksum, directory):
        raise SigningPairError('PE layout changed during signing')
    if struct.unpack_from('<II', unsigned, directory) != (0, 0):
        raise SigningPairError('Input already has a certificate table')
    offset, size = struct.unpack_from('<II', signed, directory)
    aligned = (len(unsigned) + 7) & ~7
    if offset != aligned or size < 16 or size % 8 or offset + size != len(signed):
        raise SigningPairError('Unexpected certificate table placement or size')
    if any(signed[len(unsigned):offset]):
        raise SigningPairError('Nonzero bytes before the certificate table')
    cursor, certificates = offset, 0
    while cursor < len(signed):
        if cursor + 8 > len(signed):
            raise SigningPairError('Truncated WIN_CERTIFICATE header')
        length, revision, kind = struct.unpack_from('<IHH', signed, cursor)
        end = cursor + ((length + 7) & ~7)
        if length <= 8 or end > len(signed) or revision != 0x200 or kind != 2:
            raise SigningPairError('Invalid Authenticode certificate entry')
        if any(signed[cursor + length:end]):
            raise SigningPairError('Nonzero certificate entry padding')
        cursor, certificates = end, certificates + 1
    left, right = bytearray(unsigned), bytearray(signed[:len(unsigned)])
    for start, length in ((checksum, 4), (directory, 8)):
        left[start:start + length] = right[start:start + length] = b'\0' * length
    if left != right:
        raise SigningPairError('Bytes outside the signing fields changed')
    return {
        'passed': True,
        'payloadIdentical': True,
        'unsignedSha256': hashlib.sha256(unsigned).hexdigest(),
        'signedSha256': hashlib.sha256(signed).hexdigest(),
        'unsignedBytes': len(unsigned),
        'signedBytes': len(signed),
        'certificateEntries': certificates,
        'cryptographicSignatureVerified': False,
        'windowsTrustVerified': False,
        'releaseAdmission': False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('unsigned', type=Path)
    parser.add_argument('signed', type=Path)
    args = parser.parse_args()
    try:
        for path in (args.unsigned, args.signed):
            if not path.is_file() or path.stat().st_size > MAX_BYTES:
                raise SigningPairError('Expected a bounded regular PE file')
        result = verify_bytes(args.unsigned.read_bytes(), args.signed.read_bytes())
    except (OSError, SigningPairError) as error:
        print(json.dumps({'passed': False, 'error': str(error)}))
        return 1
    print(json.dumps(result))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
