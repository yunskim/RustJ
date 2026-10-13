"""Check migrated Korean project ownership, links, and source preservation."""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
NAV = '[전체 목차](../../../PROJECT.ko.md) | [English reference](../../../PROJECT.md)\n\n'

def normalize(text):
    # Local links are relocated; labels and external provenance remain exact.
    return re.sub(r'(?<=\]\()([^\s)]+)(?=\))',
                  lambda m: m.group() if re.match(r'^[a-zA-Z][\w+.-]*:', m.group()) else 'LOCAL', text)

def main():
    manifest = json.loads((ROOT / 'reports/project-modularization.json').read_text())
    source = subprocess.check_output(
        ['git', 'show', manifest['source_commit'] + ':PROJECT.ko.md'], cwd=ROOT).decode().replace('\r\n', '\n')
    assert hashlib.sha256(source.encode()).hexdigest() == manifest['source_sha256']
    docs = [ROOT / 'PROJECT.ko.md']
    cursor = 0
    links = 0
    for part in manifest['parts']:
        assert part['start'] == cursor
        cursor = part['end']
        original = source[part['start']:part['end']]
        assert hashlib.sha256(original.encode()).hexdigest() == part['sha256']
        path = ROOT / part['path']
        text = path.read_text()
        assert text.startswith(NAV), path
        body = text[len(NAV):]
        for anchor in part['added_anchors']:
            body = body.replace(f'<a id="{anchor}"></a>\n', '', 1)
        if '--verify-source' in sys.argv:
            assert normalize(body) == normalize(original), f'changed source content: {path}'
        docs.append(path)
    assert cursor == len(source)
    assert len(docs) == 41
    errors = []
    for path in sorted(set(docs) | {p for p in ROOT.rglob('*.md') if '.git' not in p.parts and 'target' not in p.parts}):
        text = path.read_text()
        fence = None
        for line in text.splitlines():
            m = re.match(r'^\s*(`{3,}|~{3,})', line)
            if m:
                char = m.group().strip()[0]
                if fence is None:
                    fence = char
                elif fence == char:
                    fence = None
        if fence is not None and path in docs:
            errors.append(f'unbalanced fence: {path.relative_to(ROOT)}')
        for url in re.findall(r'\]\(([^\s)]+)\)', text):
            if re.match(r'^[a-zA-Z][\w+.-]*:', url) or url.startswith('//'):
                continue
            name, _, anchor = unquote(url).partition('#')
            dest = (path.parent / name).resolve() if name else path
            links += 1
            if not dest.exists():
                errors.append(f'missing file: {path.name}: {url}')
            elif anchor and dest in docs:
                if f'<a id="{anchor}"></a>' not in dest.read_text():
                    errors.append(f'missing anchor: {path.name}: {url}')
    if errors:
        print('\n'.join(errors), file=sys.stderr)
        return 1
    proof = 'complete source preservation' if '--verify-source' in sys.argv else 'pinned source manifest'
    print(f'PASS: {len(manifest["parts"])} modules, {links} local links, {proof}')
    return 0

if __name__ == '__main__':
    sys.exit(main())
