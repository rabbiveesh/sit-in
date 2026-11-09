"""Cut a GM .sf2 down to the band: python3 scripts/subset-sf2.py FluidR3_GM.sf2 web/band.sf2 22050
Keeps the presets named in KEEP, their instruments and samples; drum-kit zones only for KEYS.
With rate (e.g. 22050), samples above it are resampled down (linear) to shrink the file."""
import struct, sys

KEEP = {(0, 0), (0, 32), (0, 56), (128, 0)}
DRUM_KEYS = {36, 38, 49, 51}  # what bandstand's font voices play: kick, snare, crash, ride
PAN = 17
SAMPLE_GENS = 53; INST_GEN = 41; KEYRANGE = 43

def chunks(b):
    i = 0
    while i < len(b):
        cid, n = b[i:i+4], struct.unpack('<I', b[i+4:i+8])[0]
        yield cid, b[i+8:i+8+n]
        i += 8 + n + (n & 1)

def lists(data):
    out = {}
    for cid, body in chunks(data[12:]):
        if cid == b'LIST':
            out[body[:4]] = {c: d for c, d in chunks(body[4:])}
    return out

def recs(b, size):
    return [b[i:i+size] for i in range(0, len(b), size)]

src = open(sys.argv[1], 'rb').read()
L = lists(src)
info, sdta, pdta = L[b'INFO'], L[b'sdta'], L[b'pdta']
smpl = sdta[b'smpl']
phdr = recs(pdta[b'phdr'], 38); pbag = recs(pdta[b'pbag'], 4); pmod = recs(pdta[b'pmod'], 10); pgen = recs(pdta[b'pgen'], 4)
inst = recs(pdta[b'inst'], 22); ibag = recs(pdta[b'ibag'], 4); imod = recs(pdta[b'imod'], 10); igen = recs(pdta[b'igen'], 4)
shdr = recs(pdta[b'shdr'], 46)
rate_cap = int(sys.argv[3]) if len(sys.argv) > 3 else 0

def gens(bags, gl, ml, i):
    g0, m0 = struct.unpack('<HH', bags[i]); g1, m1 = struct.unpack('<HH', bags[i+1])
    return [struct.unpack('<HH', x) for x in gl[g0:g1]], ml[m0:m1]

def keyrange(gs):
    for o, a in gs:
        if o == KEYRANGE:
            return a & 0xff, a >> 8
    return 0, 127

# Presets -> zones
out_p = []  # (hdr fields, [ (gens, mods) ])
used_inst = []
for i in range(len(phdr) - 1):
    name, prog, bank, bag0 = struct.unpack('<20sHHH', phdr[i][:26])
    if (bank, prog) not in KEEP: continue
    bag1 = struct.unpack('<H', phdr[i+1][24:26])[0]
    zones = []
    for b in range(bag0, bag1):
        gs, ms = gens(pbag, pgen, pmod, b)
        if bank == 128:
            lo, hi = keyrange(gs)
            if not any(lo <= k <= hi for k in DRUM_KEYS): continue
        for o, a in gs:
            if o == INST_GEN and a not in used_inst: used_inst.append(a)
        zones.append((gs, ms))
    out_p.append((phdr[i], bank, zones))
print('presets', [(b, struct.unpack('<20sH', h[:22])) for h, b, _ in out_p])

drum_inst = set()
for h, bank, zones in out_p:
    if bank == 128:
        for gs, _ in zones:
            for o, a in gs:
                if o == INST_GEN: drum_inst.add(a)

out_i = []; used_s = []
for a in used_inst:
    bag0 = struct.unpack('<H', inst[a][20:22])[0]; bag1 = struct.unpack('<H', inst[a+1][20:22])[0]
    zones = []
    for b in range(bag0, bag1):
        gs, ms = gens(ibag, igen, imod, b)
        if a in drum_inst:
            lo, hi = keyrange(gs)
            if not any(lo <= k <= hi for k in DRUM_KEYS) and any(o == SAMPLE_GENS for o, _ in gs): continue
        sid = [v for o, v in gs if o == SAMPLE_GENS]
        if sid and struct.unpack('<H', shdr[sid[0]][44:46])[0] == 2: continue  # right channel: mono
        gs = [(o, v) for o, v in gs if o != PAN or not sid]
        for o, v in gs:
            if o == SAMPLE_GENS and v not in used_s: used_s.append(v)
        zones.append((gs, ms))
    out_i.append((inst[a][:20], zones))

empty = {a for a, (_, z) in zip(used_inst, out_i) if not any(o == SAMPLE_GENS for gs, _ in z for o, _ in gs)}
keep = [(a, x) for a, x in zip(used_inst, out_i) if a not in empty]
used_inst = [a for a, _ in keep]; out_i = [x for _, x in keep]
out_p = [(h, b, [z for z in zones if not any(o == INST_GEN and v in empty for o, v in z[0])]) for h, b, zones in out_p]
used_s = [v for _, z in out_i for gs, _ in z for o, v in gs if o == SAMPLE_GENS]
used_s = list(dict.fromkeys(used_s))
# Samples
k = 0
while k < len(used_s):
    link = struct.unpack('<H', shdr[used_s[k]][42:44])[0]; typ = struct.unpack('<H', shdr[used_s[k]][44:46])[0]
    pass
    k += 1

new_smpl = bytearray(); new_shdr = []; smap = {}
import array
for n, s in enumerate(used_s):
    name, start, end, ls, le, rate, pitch, corr, link, typ = struct.unpack('<20sIIIIIBbHH', shdr[s])
    data = array.array('h', smpl[start*2:end*2])
    f = 1
    if rate_cap and rate > rate_cap:
        import numpy as np
        from fractions import Fraction
        from scipy.signal import resample_poly
        fr = Fraction(rate_cap, rate).limit_denominator(1000)
        f = rate / rate_cap
        y = resample_poly(np.frombuffer(data.tobytes(), dtype='<i2').astype(np.float64), fr.numerator, fr.denominator)
        data = array.array('h', np.clip(np.round(y), -32768, 32767).astype('<i2').tobytes())
        ls2, le2 = round((ls-start)*fr), round((le-start)*fr)
        rate = round(rate * fr)
    else:
        ls2, le2 = ls-start, le-start
    base = len(new_smpl)//2
    new_smpl += data.tobytes() + b'\0' * 92  # 46 zero samples after each
    smap[s] = n
    new_shdr.append([name, base, base+len(data), base+ls2, base+le2, rate, pitch, corr, link, typ])
for h in new_shdr:
    h[8], h[9] = 0, 1
shdr_b = b''.join(struct.pack('<20sIIIIIBbHH', *h) for h in new_shdr) + struct.pack('<20sIIIIIBbHH', b'EOS', 0,0,0,0,0,0,0,0,0)

imap = {a: n for n, a in enumerate(used_inst)}
def write_zones(zonelist, remap_gen, remap):
    bags = b''; gl = b''; ml = b''; gi = mi = 0
    for zones in zonelist:
        for gs, ms in zones:
            bags += struct.pack('<HH', gi, mi)
            for o, a in gs:
                gl += struct.pack('<HH', o, remap[a] if o == remap_gen else a); gi += 1
            for m in ms: ml += m; mi += 1
    bags += struct.pack('<HH', gi, mi)
    return bags, gl + b'\0'*4, ml + b'\0'*10

inst_b = b''; bi = 0
for name, zones in out_i:
    inst_b += name + struct.pack('<H', bi); bi += len(zones)
inst_b += b'EOI'.ljust(20, b'\0') + struct.pack('<H', bi)
ibag_b, igen_b, imod_b = write_zones([z for _, z in out_i], SAMPLE_GENS, smap)

phdr_b = b''; bi = 0
for h, bank, zones in out_p:
    phdr_b += h[:24] + struct.pack('<H', bi) + h[26:]; bi += len(zones)
phdr_b += b'EOP'.ljust(20, b'\0') + struct.pack('<HHH', 0, 0, bi) + b'\0'*12
pbag_b, pgen_b, pmod_b = write_zones([z for _, _, z in out_p], INST_GEN, imap)

def ck(cid, b): return cid + struct.pack('<I', len(b)) + b + (b'\0' if len(b) & 1 else b'')
def lst(t, parts): body = t + b''.join(ck(c, d) for c, d in parts); return ck(b'LIST', body)
info_parts = [(c, d) for c, d in info.items()]
out = b'sfbk' + lst(b'INFO', info_parts) + lst(b'sdta', [(b'smpl', bytes(new_smpl))]) + lst(b'pdta', [
    (b'phdr', phdr_b), (b'pbag', pbag_b), (b'pmod', pmod_b), (b'pgen', pgen_b),
    (b'inst', inst_b), (b'ibag', ibag_b), (b'imod', imod_b), (b'igen', igen_b), (b'shdr', shdr_b)])
open(sys.argv[2], 'wb').write(b'RIFF' + struct.pack('<I', len(out)) + out)
print('samples', len(used_s), 'size', len(out)+8)
