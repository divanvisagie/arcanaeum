
# from https://raw.githubusercontent.com/nycz/FaceTransfer/master/extract.py# 
"""
This module contains all low-level game independant functions used to
manipulate and extract data in save game files.
"""

from collections import OrderedDict
import struct
import traceback
import zlib

from typing import Any, Dict, List, Set, Tuple

from common import GameError

# ========= Encode/decode functions ==========================================

def uint8(i: int, data: bytes) -> Tuple[int, int]:
    return 1, struct.unpack('B', data[i:i+1])[0]

def encode_uint8(data: int) -> bytes:
    return struct.pack('B', data)

def uint16(i: int, data: bytes) -> Tuple[int, int]:
    return 2, struct.unpack('H', data[i:i+2])[0]

def encode_uint16(data: int) -> bytes:
    return struct.pack('H', data)

def uint32(i: int, data: bytes) -> Tuple[int, int]:
    return 4, struct.unpack('I', data[i:i+4])[0]

def encode_uint32(data: int) -> bytes:
    return struct.pack('I', data)

def float32(i: int, data: bytes) -> Tuple[int, bytes]:
    # Since floats lose a shitload of precision, lets just go with bytes for now
    return 4, data[i:i+4] #struct.unpack('f', data[i:i+4])[0]

def encode_float32(data: bytes) -> bytes:
    return data #struct.pack('f', data)

def vsval(i: int, data: bytes) -> Tuple[int, int]:
    # The two rightmost bits of the first byte decides the length of the var
    size = data[i] & 0b11
    if size == 0:
        # uint8
        return 1, data[i] >> 2
    elif size == 1:
        # uint16
        return 2, (data[i] | (data[i+1] << 8)) >> 2
    elif size == 2:
        # uint32
        return 3, (data[i] | (data[i+1] << 8) | (data[i+2] << 16)) >> 2

def encode_vsval(data: int) -> bytes:
    if data < 0x40:
        # uint8
        return bytes([data << 2])
    elif data < 0x4000:
        # uint16
        data = (data << 2) + 1
        return bytes([data & 255, data >> 8])
    else:
        # uint32
        data = (data << 2) + 2
        return bytes([data & 255, (data >> 8) & 255, (data >> 16) & 255])

def wstring(i: int, data: bytes) -> Tuple[int, str]:
    _, length = uint16(i, data)
    return 2 + length, data[i+2:i+2+length].decode('cp1252')

def encode_wstring(data: str) -> bytes:
    btext = data.encode('cp1252')
    length = encode_uint16(len(btext))
    return length + btext

def bytes_(i: int, data: bytes, chunksize=1, length: int = None, end: int = None) -> Tuple[int, bytes]:
    if length is not None:
        endpoint = i+length*chunksize
    elif end is not None:
        endpoint = end
    else:
        raise Exception('Invalid arguments to bytes_ typefunc')
    return endpoint-i, data[i:endpoint]

def encode_bytes(data: bytes) -> bytes:
    return data

def formids(i: int, data: bytes, num=0) -> Tuple[int, bytes]:
    return num*4, data[i:i+num*4]

def encode_formids(data: bytes) -> bytes:
    return data

def refids(i: int, data: bytes, num=0) -> Tuple[int, bytes]:
    return num*3, data[i:i+num*3]

def encode_refids(data: bytes) -> bytes:
    return data

def screenshot(i: int, data: bytes, width=0, height=0, colorlength=0) -> Tuple[int, bytes]:
    assert width*height > 0 and colorlength > 0
    return width*height*colorlength, data[i:i+width*height*colorlength]

def encode_screenshot(data: bytes) -> bytes:
    return data

# ========= Misc functions ===================================================

def flags(i: int, data: bytes) -> Set[int]:
    return {n for n, x in enumerate(bin(uint32(i, data)[1])[2:][::-1]) if x == '1'}

def encode_flags(flags: Set[int]) -> bytes:
    return encode_uint32(sum(pow(2, x) for x in flags))

# ========= Main functions ===================================================

mainlayout = [
    (bytes_, 'magic', {'length': 12, 'game': 'fallout4'}),
    (bytes_, 'magic', {'length': 13, 'game': 'skyrim'}),
    (uint32, 'headersize', {}),
    # Header
    (uint32, 'version', {}),
    (uint32, 'savenumber', {}),
    (wstring, 'playername', {}),
    (uint32, 'playerlevel', {}),
    (wstring, 'playerlocation', {}),
    (wstring, 'gamedate', {}),
    (wstring, 'playerraceeditorid', {}),
    (uint16, 'playersex', {}),
    (float32, 'playercurexp', {}),
    (float32, 'playerlvlupexp', {}),
    (bytes_, 'filetime', {'length': 8}),
    # Screenshot
    (uint32, 'shotwidth', {}),
    (uint32, 'shotheight', {}),
    (screenshot, 'screenshotdata', {'width': 'shotwidth', 'height': 'shotheight', 'colorlength': 4, 'game': 'fallout4'}),
    (screenshot, 'screenshotdata', {'width': 'shotwidth', 'height': 'shotheight', 'colorlength': 3, 'game': 'skyrim'}),
    # Misc stuff
    (uint8, 'formversion', {}),
    (wstring, 'gameversion', {'game': 'fallout4'}),
    (uint32, 'plugininfosize', {}),
    (bytes_, 'plugininfo', {'length': 'plugininfosize'}),
    # File location table
    (uint32, 'formidarraycountoffset', {}),
    (uint32, 'unknowntable3offset', {}),
    (uint32, 'globaldatatable1offset', {}),
    (uint32, 'globaldatatable2offset', {}),
    (uint32, 'changeformsoffset', {}),
    (uint32, 'globaldatatable3offset', {}),
    (bytes_, 'globaldatatablecounts', {'length': 12}),
    (uint32, 'changeformcount', {}),
    (bytes_, 'flttail', {'length': 15*4}),
    # Data tables
    (bytes_, 'globaldatatable1', {'end': 'globaldatatable2offset'}),
    (bytes_, 'globaldatatable2', {'end': 'changeformsoffset'}),
    (bytes_, 'changeforms', {'end': 'globaldatatable3offset'}),
    (bytes_, 'globaldatatable3', {'end': 'formidarraycountoffset'}),
    (uint32, 'formidarraycount', {}),
    (formids, 'formidarray', {'num': 'formidarraycount'}),
    (uint32, 'visitedworldspacearraycount', {}),
    (formids, 'visitedworldspacearray', {'num': 'visitedworldspacearraycount'}),
    (uint32, 'unknown3tablesize', {}),
    (bytes_, 'unknown3table', {'length': 'unknown3tablesize'})
]

skyrimplayerlayout = [
    # Flag 1
    (bytes_, 'actorbasedata', {'length': 24, 'flag': 1}),
    # Flag 6 (factions)
    (uint8, 'factionsize', {'flag': 6}),
    (bytes_, 'factions', {'length': 'factionsize', 'flag': 6}),
    # Flag 4 (spells and shouts)
    (vsval, 'spellcount', {'flag': 4}),
    (refids, 'spells', {'num': 'spellcount', 'flag': 4}),
    (bytes_, 'unknown0', {'length': 1, 'flag': 4}),
    (vsval, 'shoutcount', {'flag': 4}),
    (refids, 'shouts', {'num': 'shoutcount', 'flag': 4}),
    # Flag 3
    (bytes_, 'aidata', {'length': 20, 'flag': 3}),
    # Flag 5 (name)
    (wstring, 'name', {'flag': 5}),
    # Flag 9
    (bytes_, 'skills', {'length': 52, 'flag': 9}),
    # Flag 12
    (refids, 'defaultoutfits', {'num': 1, 'flag': 12}),
    # Flag 25
    (refids, 'race', {'num': 2, 'flag': 25}),
    # Flag 11 (face)
    (bytes_, 'unknown1', {'length': 1, 'flag': 11}),
    (refids, 'haircolor', {'num': 1, 'flag': 11}),
    (bytes_, 'skincolor', {'length': 3, 'flag': 11}),
    (bytes_, 'unknown2', {'length': 1, 'flag': 11}),
    (refids, 'headtexture', {'num': 1, 'flag': 11}),
    (vsval, 'headpartcount', {'flag': 11}),
    (refids, 'headparts', {'num': 'headpartcount', 'flag': 11}),
    (bytes_, 'unknown3', {'length': 5, 'flag': 11}),
    (bytes_, 'facemorphvalues', {'length': 76, 'flag': 11}),
    (uint32, 'unknown4', {'flag': 11}),
    (uint32, 'nose', {'flag': 11}),
    (uint32, 'unknown5', {'flag': 11}),
    (uint32, 'eyes', {'flag': 11}),
    (uint32, 'mouth', {'flag': 11}),
    # Flag 24 (gender)
    (uint8, 'gender', {'flag': 24})
]

fallout4playerlayout = [
    # Flag 1
    (bytes_, 'flag1data', {'length': 20, 'flag': 1}),
    # Flag 6 (factions)
    (uint8, 'factionsize', {'flag': 6}),
    (bytes_, 'factions', {'length': 'factionsize', 'flag': 6}),
    # Flag 5 (name)
    (wstring, 'name', {'flag': 5}),
    # Flag 24 (gender)
    (uint8, 'gender', {'flag': 24}),
    # Flag 11 (headparts)
    (bytes_, 'flag11unknown1', {'length': 1, 'flag': 11}),
    (refids, 'headpart1', {'num': 1, 'flag': 11}),
    (bytes_, 'unknowncolor', {'length': 4, 'flag': 11}),
    (refids, 'headpart2', {'num': 1, 'flag': 11}),
    (vsval, 'headpartcount', {'flag': 11}),
    (refids, 'headparts', {'num': 'headpartcount', 'flag': 11}),
    # This int is really a bool and decides if the tetitend stuff is in or not
    (uint8, 'tetitendpresent', {'flag': 11}),
    (uint32, 'tetitendsize', {'ispresent': 'tetitendpresent', 'flag': 11}),
    (bytes_, 'tetitend', {'length': 'tetitendsize', 'chunksize': 8, 'ispresent': 'tetitendpresent', 'flag': 11}),
    (uint32, 'facesliderssize', {'flag': 11}),
    (bytes_, 'facesliders', {'length': 'facesliderssize', 'chunksize': 40, 'flag': 11}),
    (uint32, 'faceextrassize', {'flag': 11}),
    (bytes_, 'faceextras', {'length': 'faceextrassize', 'chunksize': 10, 'flag': 11}),
    # Flag 14 (body stuff)
    (uint32, 'bodyunknowncount', {'flag': 14}),
    (bytes_, 'bodyunknown', {'length': 'bodyunknowncount', 'chunksize': 4, 'flag': 14}),
    (float32, 'bodysliderthin', {'flag': 14}),
    (float32, 'bodyslidermuscular', {'flag': 14}),
    (float32, 'bodysliderlarge', {'flag': 14})
]

def merge_player(sourcedata, sourceflags, targetdata, targetflags, game):
    """
    Take the facial data from the sourcedata and apply it onto the targetdata.
    Return a valid player data dict like the one from parse_player coupled
    with the updated flags.
    """
    if game == 'skyrim':
        pass
    elif game == 'fallout4':
        copyflags = [11,14]
        flagdata = {}
        # Get the names of all parts related to face/body (flag 11 and 14)
        for flag in copyflags:
            flagdata[flag] = OrderedDict(
                    [(name,args) for func, name, args in fallout4playerlayout
                     if args.get('flag', -1) == flag]
            )
        # Generate a new dict to hopefully not fuck everything up due to mutability
        newflags = targetflags - set(copyflags)
        # This is a dict of all stuff from targetdata but the face/body stuff
        newdata = OrderedDict([(k,v) for k,v in targetdata.items()
                               if k not in flagdata[11] and k not in flagdata[14]])
        # Copy the data from the old
        for flag in copyflags:
            if flag in sourceflags:
                for k, args in flagdata[flag].items():
                    if 'ispresent' in args and sourcedata[args['ispresent']] == False:
                        continue
                    newdata[k] = sourcedata[k]
                newflags.add(flag)
        return newdata, newflags



def parse_player(rawdata: bytes, flags: List[int], game: str):
    """
    I'm tired af and this is the same shit as the other parse_x functions.
    In goes some bytes and out comes a nice dict you can do shit with.
    The flags should be in the format you get from parse_changeforms.
    """
    if game == 'skyrim':
        layout = skyrimplayerlayout
    elif game == 'fallout4':
        layout = fallout4playerlayout
    data = OrderedDict() # type: Dict[str, Any]
    i = 0
    for typefunc, key, rawargs in layout:
        # Skip flag-specific lines if the flag isn't active
        if 'flag' in rawargs and rawargs['flag'] not in flags:
            continue
        if 'ispresent' in rawargs and data[rawargs['ispresent']] == False:
            continue
        args = {k: data[v] if isinstance(v, str) else v
                for k,v in rawargs.items() if k not in ('flag', 'ispresent')}
        try:
            offset, data[key] = typefunc(i, rawdata, **args)
        except Exception as e:
            print('ERROR IN KEY:', key)
            raise
        i += offset
    # Make sure nothing is dropped
    assert i == len(rawdata)
    return data


def encode_player(data: Dict[str, Any], game: str):
    """
    In goes a nice player dict and out goes a nice array of bytes ready to be
    dumped in an unsuspecting changeform dict. Woo.
    """
    if game == 'skyrim':
        layout = skyrimplayerlayout
    elif game == 'fallout4':
        layout = fallout4playerlayout
    rawdata = bytes()
    def encodefunc(f):
        return globals()['encode_' + f.__name__.rstrip('_')]
    funcs = {name:encodefunc(func) for func, name, args in layout}
    for key, value in data.items():
        rawvalue = funcs[key](value)
        rawdata += rawvalue
    return rawdata


def parse_changeforms(rawdata: bytes, refidnr=7):
    """
    Convert the changeforms table to a useful dict with the different parts of
    the player's changeform and the preceding and succeeding bytes.

    The dict it returns is ready to be passed to encode_changeforms to convert
    it back to bytes.
    """
    def uint(b, sizeflag):
        return struct.unpack(['B', 'H', 'I'][sizeflag], b)[0]
    i = 0
    cfstart = 0
    data = OrderedDict() # type: Dict[str, Any]
    # Go through the changeforms until the player is found
    while True:
        cfstart = i
        refid = rawdata[i:i+3]
        changeflags = flags(i+3, rawdata)
        _, cftype = uint8(i+7, rawdata)
        _, version = uint8(i+8, rawdata)
        i += 9
        lnsize = [1,2,4][cftype >> 6]
        reallength = uint(rawdata[i:i+lnsize], cftype >> 6)
        uncompressedlength = uint(rawdata[i+lnsize:i+lnsize*2], cftype >> 6)
        i += lnsize * 2 + reallength
        # This is the players refid
        if refid == bytes([64,0,refidnr]):
            data['changeformshead'] = rawdata[:cfstart]
            data['playerrefid'] = refid
            data['playerchangeflags'] = changeflags
            data['playercftype'] = cftype & 63
            data['playerversion'] = version
            data['playerreallength'] = reallength
            data['playeruncompressedlength'] = uncompressedlength
            if uncompressedlength:
                data['playerdata'] = zlib.decompress(rawdata[i-reallength:i])
            else:
                data['playerdata'] = rawdata[i-reallength:i]
            data['changeformstail'] = rawdata[i:]
            return data


def encode_changeforms(data: Dict[str, Any]) -> bytes:
    """
    Convert the dict with data from the changeform struct back into a byte
    object, ready to be inserted into the main save data dict.

    This functions takes care of calculating the lengths of the different
    parts in it so the playeruncompressedlength and playerreallength should
    not be modified outside of this function.
    """
    # Only compress the data if the data was compressed before
    if data['playeruncompressedlength']:
        playerdata = zlib.compress(data['playerdata'])
        uncompressedlength = len(data['playerdata'])
    else:
        playerdata = data['playerdata']
        uncompressedlength = 0
    reallength = len(playerdata)
    cftype = data['playercftype']
    # Fix the whole thing with variable uint sizes for the data lengths
    if reallength > 0xffff or uncompressedlength > 0xffff:
        cftype |= 192
        reallength = encode_uint32(reallength)
        uncompressedlength = encode_uint32(uncompressedlength)
    elif reallength > 0xff or uncompressedlength > 0xff:
        cftype |= 64
        reallength = encode_uint16(reallength)
        uncompressedlength = encode_uint16(uncompressedlength)
    else:
        reallength = encode_uint8(reallength)
        uncompressedlength = encode_uint8(uncompressedlength)
    # Build the actual bytechunk
    rawdata = data['changeformshead'] + data['playerrefid']
    rawdata += encode_flags(data['playerchangeflags'])
    rawdata += encode_uint8(cftype)
    rawdata += encode_uint8(data['playerversion'])
    rawdata += reallength + uncompressedlength + playerdata
    rawdata += data['changeformstail']
    return rawdata


def parse_savedata(rawdata: bytes) -> Tuple[str, Dict[str, Any]]:
    """
    Convert the entirety of a save file (as a bytes object) into an ordered
    dict with all the data from the save file in a more accessible format.

    The dict is also ready to be passed to encode_savedata to be converted
    back to a save file.
    """
    if rawdata[:13] == b'TESV_SAVEGAME':
        game = 'skyrim'
    elif rawdata[:12] == b'FO4_SAVEGAME':
        game = 'fallout4'
    else:
        raise GameError('Game not recognized. Magic is "{}"'.format(rawdata[:12].decode()))
    data = OrderedDict() # type: Dict[str, Any]
    i = 0
    for typefunc, key, rawargs in mainlayout:
        # Skip game-specific lines for the wrong game
        if rawargs.get('game', game) != game:
            continue
        # All string args should be variable names and replaced by
        # their values, and the key 'game' should be removed
        # DO NOT EDIT THE FUCKING ARGS DICT DIRECTLY
        # OR EVERYTHING WILL BLOW UP b/c fuck mutability
        args = {k: data[v] if isinstance(v, str) else v
                for k,v in rawargs.items() if k != 'game'}
        offset, data[key] = typefunc(i, rawdata, **args)
        i += offset
    assert i == len(rawdata)
    return game, data

def update_savedata_offsets(data: Dict[str, Any]) -> None:
    """
    Make sure that the offsets to the various data tables are correct in
    relation to the (possibly) new/changed tables. Plugin Info, changeForms
    and formIDArray may all have changed size and the preceding offsets
    should be change accordingly.

    NOTE that this changes the data dict in place and doesn't return anything.
    """
    # changeForms
    oldcflength = data['globaldatatable3offset'] - data['changeformsoffset']
    if len(data['changeforms']) != oldcflength:
        cflengthdiff = len(data['changeforms']) - oldcflength
        data['globaldatatable3offset'] += cflengthdiff
        data['formidarraycountoffset'] += cflengthdiff
        data['unknowntable3offset'] += cflengthdiff
    # formIDArray
    # TODO: this and Plugin Info


def encode_savedata(data: Dict[str, Any]) -> bytes:
    """
    Take a dictionary with the valid structure of a save file (aka the right
    offsets etc) and merge it into bytes ready to be written to the disc as a
    save file.
    """
    if data['magic'] == b'TESV_SAVEGAME':
        game = 'skyrim'
    elif data['magic'] == b'FO4_SAVEGAME':
        game = 'fallout4'
    else:
        raise GameError('Game not recognized. Magic is "{}"'.format(data['magic'].decode()))
    update_savedata_offsets(data)
    rawdata = bytes()
    def encodefunc(f):
        return globals()['encode_' + f.__name__.rstrip('_')]
    funcs = {name:encodefunc(func) for func, name, args in mainlayout
             if args.get('game', game) == game}
    for key, value in data.items():
        rawvalue = funcs[key](value)
        rawdata += rawvalue
    return rawdata