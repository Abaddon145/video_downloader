"""Bounded delivery scan. Reports locations and counts, never matching secret values."""
from pathlib import Path
import hashlib,json,re,subprocess,sys
root=Path.cwd()
names=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=root).decode('utf-8').split('\0')
files={root/name for name in names if name and (root/name).is_file()}
files.update(p for p in (root/'dist').rglob('*') if p.is_file())
pattern=re.compile(rb'\bsk-[A-Za-z0-9_-]{24,}\b')
# Exact synthetic literals used by existing loopback/DPAPI tests; never ignore an entire file.
reviewed_fixtures={('src-tauri/src/ai/analysis_tests.rs', 'f8c071995cb7c6867144f7b5cd4eca2981f20db7510105ffe4f0598d0e684f4a'), ('src-tauri/src/ai/note_organize_tests.rs', '8c061a49954de0a17139ef2da666a10593054449556afe22eee4ef166690ff12'), ('src-tauri/src/ai/tests.rs', '97240756922ad188339f3523bd261c7d09a34e41ddf3fbedf78fa67c924c304b'), ('src-tauri/src/ai/tests.rs', '2e14a722b9548c4b6c7ca444788211abce440feafe9c0b66fe72c2ca792b7758'), ('src-tauri/src/ai/reference_tests.rs', '35342437ae6a94afc8d29749d59bc58337966bb16d417ce2dbf02ba868ece0a5')}
findings=[];synthetic=[];scanned=0
for path in sorted(files):
    if path.suffix.lower() not in {'.rs','.ts','.tsx','.js','.mjs','.cjs','.json','.toml','.md','.txt','.html','.css','.sql','.py','.yml','.yaml','.ps1'}:continue
    data=path.read_bytes();scanned+=1
    for match in pattern.finditer(data):
        location={'path':path.relative_to(root).as_posix(),'line':data[:match.start()].count(b'\n')+1}
        (synthetic if (location['path'],hashlib.sha256(match.group()).hexdigest()) in reviewed_fixtures else findings).append(location)
out=root/'docs/evidence/v0.5.2/phase7/security.json';out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(json.dumps({'scope':'Git source/evidence plus generated dist text; not all user data','scannedFiles':scanned,'unreviewedMatches':len(findings),'reviewedSyntheticTestLiterals':len(synthetic),'locations':findings,'reviewedLocations':synthetic},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(f'Scanned {scanned} delivery text files; {len(findings)} unreviewed plaintext-key matches; {len(synthetic)} exact synthetic test literals (values never printed).')
sys.exit(bool(findings))
