#!/usr/bin/env bash
# Pacote Linux: o OCCT permanece substituível, com nomes de carregamento locais.
# O hospedeiro fornece glibc, runtime C++ e a pilha de janela/GPU.
set -euo pipefail
BIN_PATH=""
OCCT_LIB_DIR=""
OUTPUT_DIR=""
GENERATE_ARCHIVE=false
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() { echo "Erro: $*" >&2; exit 1; }
show_help() {
    cat <<EOF
Uso: $0 [OPÇÕES]
  --bin CAMINHO            Executável compilado com OCCT
  --occt-lib-dir CAMINHO   Diretório do SDK, usado também na resolução
  --output CAMINHO         Destino NOVO (padrão: dist/3dnator-linux-x86_64)
  --archive                Gera também um NOVO arquivo .tar.gz
  -h, --help               Exibe esta ajuda
Os caminhos informados são relativos ao diretório de invocação.
EOF
}
while [[ $# -gt 0 ]]; do
    case "$1" in
        --bin|--occt-lib-dir|--output)
            [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail "Falta o valor de $1."
            case "$1" in
                --bin) BIN_PATH="$2" ;;
                --occt-lib-dir) OCCT_LIB_DIR="$2" ;;
                --output) OUTPUT_DIR="$2" ;;
            esac
            shift 2 ;;
        --archive) GENERATE_ARCHIVE=true; shift ;;
        -h|--help) show_help; exit 0 ;;
        *) fail "Argumento desconhecido: $1. Use --help." ;;
    esac
done
BIN_PATH="${BIN_PATH:-$REPO_ROOT/target/release/nator-prototype}"
[[ -f "$BIN_PATH" && -x "$BIN_PATH" ]] || fail "Executável ausente: $BIN_PATH. Use --bin ou compile em release."
BIN_PATH="$(realpath -e -- "$BIN_PATH")"
OUTPUT_DIR="${OUTPUT_DIR:-$REPO_ROOT/dist/3dnator-linux-x86_64}"
# Nunca limpar um caminho recebido do usuário, nem sobrescrever pacotes anteriores.
[[ ! -e "$OUTPUT_DIR" && ! -L "$OUTPUT_DIR" ]] || fail "O destino já existe: $OUTPUT_DIR. Escolha um diretório novo."
OUTPUT_DIR="$(realpath -m -- "$OUTPUT_DIR")"
if $GENERATE_ARCHIVE; then
    [[ ! -e "$OUTPUT_DIR.tar.gz" && ! -L "$OUTPUT_DIR.tar.gz" ]] || fail "O arquivo $OUTPUT_DIR.tar.gz já existe."
fi
if [[ -n "$OCCT_LIB_DIR" ]]; then
    [[ -d "$OCCT_LIB_DIR" ]] || fail "Diretório OCCT inexistente: $OCCT_LIB_DIR."
    OCCT_LIB_DIR="$(realpath -e -- "$OCCT_LIB_DIR")"
    export LD_LIBRARY_PATH="$OCCT_LIB_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
for doc in LICENSE THIRD_PARTY.md docs/PACKAGE_README.md licenses/OCCT/LICENSE_LGPL_21.txt licenses/OCCT/OCCT_LGPL_EXCEPTION.txt; do
    [[ -f "$REPO_ROOT/$doc" ]] || fail "Documento obrigatório ausente: $doc."
done
# ldd resolve também dependências transitivas. Rejeitar ausências antes de criar
# a saída impede anunciar um pacote utilizável quando o SDK está incompleto.
DEPENDENCIES="$(LC_ALL=C ldd "$BIN_PATH")" || fail "Não foi possível analisar o executável com ldd."
[[ "$DEPENDENCIES" != *"not found"* ]] || fail "Dependências não encontradas; configure --occt-lib-dir ou LD_LIBRARY_PATH: $DEPENDENCIES"
LIB_NAMES=()
LIB_PATHS=()
HOST_LIBS=()
OCCT_COUNT=0
while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*([^[:space:]]+)[[:space:]]+\=\>[[:space:]]+(.+)[[:space:]]+\(0x[[:xdigit:]]+\)$ ]]; then
        name="${BASH_REMATCH[1]}"
        path="${BASH_REMATCH[2]}"
        # TBB é opcional no SDK. As demais bibliotecas são requisitos do host,
        # registrados no manifesto; não se deve redistribuir a pilha de drivers.
        case "$name" in
            libTK*.so*) OCCT_COUNT=$((OCCT_COUNT + 1)) ;;
            libtbb*.so*) ;;
            *) HOST_LIBS+=("$name"); continue ;;
        esac
        [[ -f "$path" ]] || fail "Biblioteca inválida: $path."
        LIB_NAMES+=("$name")
        LIB_PATHS+=("$path")
    fi
done <<< "$DEPENDENCIES"
[[ "$OCCT_COUNT" -gt 0 ]] || fail "O executável não possui vínculo dinâmico com OCCT."
mkdir -p -- "$(dirname "$OUTPUT_DIR")"
mkdir -- "$OUTPUT_DIR"
mkdir -- "$OUTPUT_DIR/bin" "$OUTPUT_DIR/lib"
cp -- "$BIN_PATH" "$OUTPUT_DIR/bin/3dnator"
chmod 755 "$OUTPUT_DIR/bin/3dnator"
for index in "${!LIB_NAMES[@]}"; do
    # Copiar o conteúdo sob o SONAME evita links absolutos para o SDK e cadeias
    # de symlinks quebradas depois de mover o pacote para outra máquina.
    cp -L -- "${LIB_PATHS[index]}" "$OUTPUT_DIR/lib/${LIB_NAMES[index]}"
done
cat <<'EOF' > "$OUTPUT_DIR/3dnator"
#!/usr/bin/env sh
# Bibliotecas locais têm precedência e podem ser substituídas pelo usuário.
set -e
BUNDLE_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
export LD_LIBRARY_PATH="$BUNDLE_DIR/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$BUNDLE_DIR/bin/3dnator" "$@"
EOF
chmod 755 "$OUTPUT_DIR/3dnator"
cp -- "$REPO_ROOT/LICENSE" "$REPO_ROOT/THIRD_PARTY.md" "$OUTPUT_DIR/"
cp -- "$REPO_ROOT/docs/PACKAGE_README.md" "$OUTPUT_DIR/README.md"
cp -R -- "$REPO_ROOT/licenses" "$OUTPUT_DIR/"
{
    echo "Bibliotecas incluídas (SONAME):"
    printf '%s\n' "${LIB_NAMES[@]}"
    echo "Bibliotecas requeridas do host (além do carregador ELF e dos drivers gráficos):"
    printf '%s\n' "${HOST_LIBS[@]}"
} > "$OUTPUT_DIR/NATIVE_DEPENDENCIES.txt"
# Este ensaio continua sendo uma validação neste host. Compatibilidade entre
# distribuições exige um runner com a base desejada.
env -i PATH=/usr/bin:/bin "$OUTPUT_DIR/3dnator" --version
if $GENERATE_ARCHIVE; then
    tar -czf "$OUTPUT_DIR.tar.gz" -C "$(dirname "$OUTPUT_DIR")" -- "$(basename "$OUTPUT_DIR")"
fi
echo "Pacote criado em $OUTPUT_DIR (${#LIB_NAMES[@]} bibliotecas)."
