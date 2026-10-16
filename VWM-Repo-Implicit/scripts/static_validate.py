from __future__ import annotations

import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
errors: list[str] = []
checks: list[str] = []

# TOML parsing and workspace/path validation.
cargo_files = sorted(ROOT.rglob('Cargo.toml'))
parsed = {}
for path in cargo_files:
    try:
        parsed[path] = tomllib.loads(path.read_text(encoding='utf-8'))
    except Exception as exc:
        errors.append(f'TOML parse failed: {path.relative_to(ROOT)}: {exc}')
checks.append(f'Parsed {len(parsed)} Cargo.toml files')

workspace = parsed.get(ROOT / 'Cargo.toml', {}).get('workspace', {})
members = workspace.get('members', [])
for member in members:
    cargo = ROOT / member / 'Cargo.toml'
    if not cargo.is_file():
        errors.append(f'Workspace member missing Cargo.toml: {member}')
checks.append(f'Validated {len(members)} workspace members')

package_names: dict[str, Path] = {}
for path, data in parsed.items():
    package = data.get('package', {})
    name = package.get('name')
    if name:
        if name in package_names:
            errors.append(
                f'Duplicate package name {name}: {package_names[name].relative_to(ROOT)} and {path.relative_to(ROOT)}'
            )
        package_names[name] = path
    for dep_table_name in ('dependencies', 'dev-dependencies', 'build-dependencies'):
        for dep_name, dep_value in data.get(dep_table_name, {}).items():
            if isinstance(dep_value, dict) and 'path' in dep_value:
                target = (path.parent / dep_value['path'] / 'Cargo.toml').resolve()
                if not target.is_file():
                    errors.append(
                        f'Path dependency missing for {dep_name} in {path.relative_to(ROOT)}: {target}'
                    )
checks.append(f'Validated {len(package_names)} unique package names and local path dependencies')

# Rust module and delimiter validation.
def strip_rust(text: str) -> str:
    out: list[str] = []
    i = 0
    state = 'code'
    block_depth = 0
    raw_hashes = 0
    while i < len(text):
        ch = text[i]
        nxt = text[i + 1] if i + 1 < len(text) else ''
        if state == 'code':
            if ch == '/' and nxt == '/':
                state = 'line_comment'; out.extend('  '); i += 2; continue
            if ch == '/' and nxt == '*':
                state = 'block_comment'; block_depth = 1; out.extend('  '); i += 2; continue
            if ch == '"':
                state = 'string'; out.append(' '); i += 1; continue
            if ch == "'":
                # Treat only obvious character literals as strings; lifetimes remain code.
                if i + 2 < len(text) and text[i + 2] == "'":
                    state = 'char'; out.append(' '); i += 1; continue
            if ch == 'r':
                match = re.match(r'r(#+)?"', text[i:])
                if match:
                    raw_hashes = len(match.group(1) or '')
                    state = 'raw'; out.extend(' ' * len(match.group(0))); i += len(match.group(0)); continue
            out.append(ch); i += 1; continue
        if state == 'line_comment':
            if ch == '\n': state = 'code'; out.append('\n')
            else: out.append(' ')
            i += 1; continue
        if state == 'block_comment':
            if ch == '/' and nxt == '*': block_depth += 1; out.extend('  '); i += 2; continue
            if ch == '*' and nxt == '/':
                block_depth -= 1; out.extend('  '); i += 2
                if block_depth == 0: state = 'code'
                continue
            out.append('\n' if ch == '\n' else ' '); i += 1; continue
        if state in ('string', 'char'):
            terminator = '"' if state == 'string' else "'"
            if ch == '\\': out.extend('  '); i += 2; continue
            if ch == terminator: state = 'code'
            out.append('\n' if ch == '\n' else ' '); i += 1; continue
        if state == 'raw':
            if ch == '"' and text[i + 1:i + 1 + raw_hashes] == '#' * raw_hashes:
                size = 1 + raw_hashes
                out.extend(' ' * size); i += size; state = 'code'; continue
            out.append('\n' if ch == '\n' else ' '); i += 1; continue
    return ''.join(out)

rust_files = sorted(ROOT.rglob('*.rs'))
for path in rust_files:
    text = path.read_text(encoding='utf-8')
    stripped = strip_rust(text)
    stack: list[tuple[str, int]] = []
    pairs = {')': '(', ']': '[', '}': '{'}
    for pos, char in enumerate(stripped):
        if char in '([{': stack.append((char, pos))
        elif char in ')]}':
            if not stack or stack[-1][0] != pairs[char]:
                errors.append(f'Unbalanced delimiter {char} in {path.relative_to(ROOT)} at byte {pos}')
                break
            stack.pop()
    if stack:
        errors.append(f'Unclosed delimiter {stack[-1][0]} in {path.relative_to(ROOT)}')

    if path.name in ('lib.rs', 'main.rs') or path.parent.name == 'src':
        for match in re.finditer(r'(?m)^\s*(?:pub\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;', stripped):
            module = match.group(1)
            direct = path.parent / f'{module}.rs'
            nested = path.parent / module / 'mod.rs'
            if not direct.is_file() and not nested.is_file():
                errors.append(f'Module {module} declared in {path.relative_to(ROOT)} has no source file')
checks.append(f'Validated delimiters and module declarations in {len(rust_files)} Rust files')

# New crate completeness and forbidden placeholders.
required = [
    'crates/vwm-implicit-core/src/contracts.rs',
    'crates/vwm-implicit-core/src/grid.rs',
    'crates/vwm-implicit-core/src/ray.rs',
    'crates/vwm-implicit-poisson/src/lib.rs',
    'crates/vwm-implicit-surface-nets/src/lib.rs',
    'crates/vwm-implicit/src/lib.rs',
    'docs/implicit-field-integration.md',
]
for rel in required:
    path = ROOT / rel
    if not path.is_file() or path.stat().st_size == 0:
        errors.append(f'Required file missing or empty: {rel}')

for path in list((ROOT / 'crates').glob('vwm-implicit*/**/*.rs')) + [ROOT / 'docs/implicit-field-integration.md']:
    if not path.is_file():
        continue
    text = path.read_text(encoding='utf-8')
    for pattern in (r'\bTODO\b', r'\bTBD\b', r'\bunimplemented!\s*\(', r'\btodo!\s*\('):
        if re.search(pattern, text, flags=re.IGNORECASE):
            errors.append(f'Forbidden placeholder pattern {pattern} in {path.relative_to(ROOT)}')
checks.append('Checked required files and placeholder patterns')

# Inventory digest for reproducibility.
manifest: list[dict[str, object]] = []
for path in sorted(p for p in ROOT.rglob('*') if p.is_file() and p.name != 'STATIC_VALIDATION.json'):
    data = path.read_bytes()
    manifest.append({
        'path': path.relative_to(ROOT).as_posix(),
        'size': len(data),
        'sha256': hashlib.sha256(data).hexdigest(),
    })

report = {
    'root': str(ROOT),
    'checks': checks,
    'errors': errors,
    'file_count': len(manifest),
    'files': manifest,
    'rust_toolchain_available': False,
    'compile_tests_executed': False,
}
(ROOT / 'STATIC_VALIDATION.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('\n'.join(checks))
print(f'Inventory files: {len(manifest)}')
if errors:
    print('ERRORS:')
    print('\n'.join(f'- {error}' for error in errors))
    sys.exit(1)
print('Static validation passed with 0 errors')
