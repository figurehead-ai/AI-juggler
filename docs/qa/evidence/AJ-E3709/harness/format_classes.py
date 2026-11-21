import json,re,sys,collections
sys.path.insert(0,'harness')
import checks,gate
OUT=sys.argv[1]
def cls(t,w):
    s=t
    fin=w.get('finish')
    if re.search(r'</?think|<\|[^>]*\|>|<\|im_|\[/?INST\]|^(assistant|user|system)\s*[:\n]',t,re.I) or re.search(r'["“”]',t): return 'C'
    if re.match(r'\s*(sure|okay|ok|here\'?s|here is|summary|certainly|the agent'+r')\b[^.]{0,0}[:,]',t,re.I) or re.match(r'\s*(sure|okay|certainly|here|summary|the message)\b.{0,25}[:,]',t,re.I): return 'A'
    if checks.non_answer(t,w.get('refused')) or re.search(r'[一-鿿぀-ヿЀ-ӿ؀-ۿ]',t) or re.match(r'\s*[-*•]\s|\s*\d+[.)]\s',t) or '\n' in t: return 'F'
    if fin=='length' or len(t)>=159: return 'B'
    flat=re.sub(r"`[^`]*`|\b[\w-]*\w\.\w[\w.]*|\be\.g\.|\bi\.e\.","X",t)
    if re.findall(r"[.!?]+\s+\S",flat): return 'E'
    if len(t.split())>20: return 'D'
    return 'G'
res={}
for tag,f in [('qwen','results-qwen3-1.7b-q8-v3.json'),('nemo','results-nemotron3-nano-4b-q4km-v3.json')]:
    d=json.load(open(f))['results']
    out=collections.defaultdict(lambda:{'n':0,'fail':0,'c':collections.Counter(),'ex':collections.defaultdict(list),'chk':collections.defaultdict(collections.Counter)})
    chooseinv=collections.Counter(); chooses=collections.Counter()
    for r in d:
        if r['choose']:
            chooses[r['corpus']]+=1
            if r['choose'].get('finish')=='length' or r['choose'].get('error') or not re.fullmatch(r'\s*\d+\s*',r['choose'].get('text') or ''): chooseinv[r['corpus']]+=1
        w=r['write']
        if not w: continue
        t=(w.get('text') or '').strip(); o=out[r['corpus']]; o['n']+=1
        st,_=gate.strip_quote_pair(t)
        ok,errs,_n=checks.format_check(st.replace('`',''))
        na=checks.non_answer(st,w.get('refused')) or w.get('error')
        if ok and w.get('finish')!='length' and not na: continue
        o['fail']+=1
        c=cls(t,w); o['c'][c]+=1
        why=list(errs)+(['finish=length'] if w.get('finish')=='length' else [])+(['non-answer'] if na else [])
        for e in why: o['chk'][c][re.sub(r'\d+','N',e)]+=1
        o['ex'][c].append((r['id'],r['rep'],w.get('finish'),t,why))
    res[tag]=(out,chooses,chooseinv)
L=['# AJ-E3709 v3 local tier: format-failure classes (draft, Sonnet; Opus to check)\n',
'Source: results-*-v3.json write drafts. A draft is format-failed when it is not counted in report_v3 `format_pass`: `checks.format_check(strip_quote_pair(text).replace("`",""))` fails (newline, list, >1 sentence, >20 words, >160 chars, empty), or finish=length, or it was dropped earlier as a non-answer (`checks.non_answer`: refusal phrase, <3 words, error). Classes assigned by script in priority order C, A, F, B, E, D, G (one class each). Choose calls are not format-counted; counts of invalid picks are shown for information.\n',
'Class key: A think/preamble, B stop/limit (finish=length or at 160-char cap), C template artefact (role/special tokens, think tags, quotation marks), D too long but complete (one sentence >20 words), E several sentences, F wrong content type (non-answer, refusal, other script, list, newline), G other.\n']
L.append('## Counts\n\n| model | corpus | drafts | format-failed | A | B | C | D | E | F | G |\n|---|---|---|---|---|---|---|---|---|---|---|')
for tag,(out,ch,ci) in res.items():
    tn=tf=0;tc=collections.Counter()
    for k,o in out.items():
        L.append(f"| {tag} | {k} | {o['n']} | {o['fail']} | "+" | ".join(str(o['c'][x]) for x in 'ABCDEFG')+" |")
        tn+=o['n'];tf+=o['fail'];tc+=o['c']
    L.append(f"| {tag} | **all** | {tn} | {tf} | "+" | ".join(str(tc[x]) for x in 'ABCDEFG')+" |")
L.append('\nChoose calls (not format-counted), invalid/length-cut picks: '+"; ".join(f"{t}: "+", ".join(f"{k} {ci[k]}/{ch[k]}" for k in ch) for t,(o,ch,ci) in res.items())+'\n')
L.append('## Rejecting checks per class\n')
for tag,(out,_,_) in res.items():
    tot=collections.defaultdict(collections.Counter)
    for o in out.values():
        for c,cc in o['chk'].items(): tot[c].update(cc)
    for c in sorted(tot): L.append(f"- {tag} class {c}: "+", ".join(f"{k} x{v}" for k,v in tot[c].most_common()))
L.append('\n## Examples (verbatim; id, rep, finish, rejecting checks)\n')
for tag,(out,_,_) in res.items():
    for c in 'ABCDEFG':
        ex=[]
        for k,o in out.items(): ex+= [(k,)+e for e in o['ex'][c]]
        L.append(f"### {tag} class {c} ({len(ex)})")
        seen=set();n=0
        for e in ex:
            if e[4] in seen: continue
            seen.add(e[4]);n+=1
            L.append(f"- [{e[0]} {e[1]} rep{e[2]} finish={e[3]}] `{e[4]}` -> {', '.join(e[5])}")
            if n==3:break
        if not ex:L.append('- none')
open(OUT,'w').write("\n".join(L)+"\n")
