#!/usr/bin/env python3
"""Coverage report for every static t() call; untranslated fallback is not counted."""
import json,re
from pathlib import Path
root=Path(__file__).resolve().parents[1]
src=(root/'src/App.tsx').read_text(encoding='utf8')
keys=set(re.findall(r'\bt\([\'\"]([^\'\"]+)[\'\"]\)',src))
comm=(root/'src/localization-community.ts').read_text(encoding='utf8')
data=json.loads(comm.split(' = ',1)[1].strip().rstrip(';'))
for lang,strings in data.items():
 missing=sorted(keys-set(strings))
 print(f'{lang}: {len(keys)-len(missing)}/{len(keys)} community strings; missing {len(missing)} (may be translated in other packs)')
