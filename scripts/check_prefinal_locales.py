#!/usr/bin/env python3
"""Audit static translation keys and achievement names in all eight Hellforge locales."""
import re,json,sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
src=(root/'src/App.tsx').read_text(encoding='utf-8')
pack=json.loads((root/'src/localization-prefinal.ts').read_text(encoding='utf-8').split(' = ',1)[1].rstrip().removesuffix(';'))
next_pack=json.loads(re.search(r'= (\{.*\});',(root/'src/localization-next.ts').read_text(encoding='utf-8'),re.S).group(1))
keys=set(re.findall(r'\bt\([\'\"]([^\'\"]+)[\'\"]\)',src))
for variable in ['hourNames','modNames','mapNames','profileNames','completedNames','launchNames']:
 match=re.search(r'const '+variable+r':.*?=\{(.*?)\};',src,re.S)
 if not match:raise SystemExit('Missing '+variable)
 keys.update(re.findall(r"\d+:\['([^']+)'",match.group(1)))
langs=['en','pl','de','fr','es','ja','zh']
print('Checking',len(keys),'keys (static UI + unique achievement titles)')
fail=False
for lang in langs:
 missing=sorted(keys-set(pack[lang])-set(next_pack.get(lang,{})))
 print(f'{lang}: {len(keys)-len(missing)}/{len(keys)} present; {len(missing)} unresolved in new+next dictionaries')
 if missing:
  fail=True
  for k in missing:print('  MISSING:',k)
if fail:sys.exit(1)
print('PASS: every tracked key has a translation in all seven non-Ukrainian locales.')
