#!/usr/bin/env python3
"""Report untranslated UI keys; exit 1 with --strict if any enabled language is incomplete.
This checks calls to t('...') and the dictionary declarations. Dynamic and bare JSX
strings require a separate review and are intentionally not counted as translated.
"""
import ast,json,re,sys
from pathlib import Path
try: import json5
except ImportError: raise SystemExit('Install json5: python -m pip install json5')
root=Path(__file__).resolve().parents[1]
src=(root/'src/App.tsx').read_text(encoding='utf8')
keys=set(re.findall(r'\bt\([\'\"]([^\'\"]+)[\'\"]\)',src))
# Locate JS object initializers by balanced brace scanning.
def get_object(name):
 m=re.search(r'const '+name+r'(?::[^=]+)?=',src)
 if not m:return {}
 start=src.find('{',m.end());depth=0;quote=None;escape=False
 for i in range(start,len(src)):
  c=src[i]
  if quote:
   if escape:escape=False
   elif c=='\\':escape=True
   elif c==quote:quote=None
  elif c in ('\"',"'",'`'):quote=c
  elif c=='{':depth+=1
  elif c=='}':
   depth-=1
   if depth==0:
    return json5.loads(src[start:i+1])
 return {}
base={name:get_object(name) for name in ['EN','OTHER','NEW_UI','EXTRA_UI','ADVANCED_UI']}
extra=json.loads(re.search(r'= (\{.*\});', (root/'src/localization-next.ts').read_text(),re.S).group(1))
print(f'Static t() keys: {len(keys)} (hardcoded JSX, dynamic strings, and achievements not included)')
for lang in ['en','pl','de','fr','es','ja','zh']:
 covered=[];missing=[]
 for k in sorted(keys):
  if (lang=='en' and base['EN'].get(k)) or any(base[name].get(lang,{}).get(k) for name in ['OTHER','NEW_UI','EXTRA_UI','ADVANCED_UI']) or extra.get(lang,{}).get(k):covered.append(k)
  else:missing.append(k)
 print(f'{lang:>2}: {len(covered):>3}/{len(keys)} translated; missing {len(missing)}')
 (root/('localization-audit-'+lang+'.txt')).write_text('\n'.join(missing)+'\n',encoding='utf8')
 if '--strict' in sys.argv and missing:sys.exit(1)
