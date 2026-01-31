#!/usr/bin/env bash
set -euo pipefail

example="${1:-bst_topk}"
mode="${2:-python}"

mode="$(printf "%s" "${mode}" | tr '[:upper:]' '[:lower:]')"
if [[ "${mode}" != "python" && "${mode}" != "rust" ]]; then
  echo "invalid mode: ${mode} (expected python or rust)" >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root_dir="$(cd "${script_dir}/.." && pwd)"

if_file="${root_dir}/examples/${example}.if"
extra_rs="${root_dir}/examples/${example}_extra.rs"
extra_py="${root_dir}/examples/${example}_extra.py"

if [[ ! -f "${if_file}" ]]; then
  echo "example IF file not found: ${if_file}" >&2
  echo "available examples: bst_topk, mini_web, mini_sql" >&2
  exit 1
fi

if ! command -v if_lang >/dev/null 2>&1; then
  echo "if_lang not found in PATH. Install with: cargo install if_lang" >&2
  exit 1
fi

if [[ "${mode}" == "python" ]]; then
  if [[ ! -f "${extra_py}" ]]; then
    echo "example extra file not found: ${extra_py}" >&2
    exit 1
  fi
  if_lang extra "${extra_py}" "${if_file}"
  exit 0
fi

if [[ ! -f "${extra_rs}" ]]; then
  echo "example extra file not found: ${extra_rs}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
cleanup() { rm -rf "${tmp_dir}"; }
trap cleanup EXIT

crate_name="$(basename "${extra_rs}" .rs)"

cat > "${tmp_dir}/Cargo.toml" <<EOF
[package]
name = "${crate_name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
if_lang = "0.1"
EOF

mkdir -p "${tmp_dir}/src"
cp "${extra_rs}" "${tmp_dir}/src/lib.rs"

(cd "${tmp_dir}" && cargo build --release)

case "$(uname -s)" in
  Darwin) lib_ext="dylib" ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT) lib_ext="dll" ;;
  *) lib_ext="so" ;;
esac

lib_path="${tmp_dir}/target/release/lib${crate_name}.${lib_ext}"

if [[ ! -f "${lib_path}" ]]; then
  echo "compiled library not found: ${lib_path}" >&2
  exit 1
fi

if_lang extra "${lib_path}" "${if_file}"
