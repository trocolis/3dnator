#!/usr/bin/env bash
# Script de empacotamento do 3dnator para Linux (x86_64).
# Reúne o binário compilado, as bibliotecas compartilhadas do OpenCASCADE (OCCT)
# e os avisos de licença em um pacote portátil independente de instalação global.

set -euo pipefail

BIN_PATH=""
OCCT_LIB_DIR=""
OUTPUT_DIR=""
GENERATE_ARCHIVE=false

mostrar_ajuda() {
    cat <<EOF
Uso: $0 [OPÇÕES]

Opções:
  --bin CAMINHO            Caminho do binário compilado (nator-prototype)
  --occt-lib-dir CAMINHO   Diretório contendo as bibliotecas dinâmicas do OCCT (libTK*.so)
  --output CAMINHO         Diretório de destino do pacote (padrão: dist/3dnator-linux-x86_64)
  --archive                Gera também um arquivo .tar.gz comprimido
  -h, --help               Exibe esta mensagem de ajuda
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bin)
            BIN_PATH="$2"
            shift 2
            ;;
        --occt-lib-dir)
            OCCT_LIB_DIR="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --archive)
            GENERATE_ARCHIVE=true
            shift
            ;;
        -h|--help)
            mostrar_ajuda
            exit 0
            ;;
        *)
            echo "Erro: argumento desconhecido: $1" >&2
            mostrar_ajuda >&2
            exit 1
            ;;
    esac
done

# Localizar a raiz do repositório
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Se o binário não foi especificado, procurar nos locais usuais de compilação
if [[ -z "$BIN_PATH" ]]; then
    for cand in \
        "target/claude-occt/release/nator-prototype" \
        "target/release/nator-prototype" \
        "target/claude-occt/debug/nator-prototype" \
        "target/debug/nator-prototype"
    do
        if [[ -x "$cand" ]]; then
            BIN_PATH="$cand"
            break
        fi
    done
fi

if [[ -z "$BIN_PATH" || ! -x "$BIN_PATH" ]]; then
    echo "Erro: nenhum executável válido encontrado. Especifique com --bin ou compile o projeto." >&2
    exit 1
fi

echo "==> Usando executável: $BIN_PATH"

# Definir diretório de saída
if [[ -z "$OUTPUT_DIR" ]]; then
    OUTPUT_DIR="$REPO_ROOT/dist/3dnator-linux-x86_64"
fi

echo "==> Preparando diretório de destino: $OUTPUT_DIR"
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/bin"
mkdir -p "$OUTPUT_DIR/lib"

# Copiar executável
cp "$BIN_PATH" "$OUTPUT_DIR/bin/3dnator"
chmod 755 "$OUTPUT_DIR/bin/3dnator"

# Identificar bibliotecas OCCT necessárias a partir do ldd
echo "==> Analisando dependências dinâmicas com ldd..."
OCCT_LIBS=$(ldd "$BIN_PATH" 2>/dev/null | grep -E "libTK" | awk '{print $3}' | grep -v "^$" || true)

COPIED_COUNT=0

if [[ -n "$OCCT_LIBS" ]]; then
    for lib in $OCCT_LIBS; do
        if [[ -f "$lib" ]]; then
            # Copiar a biblioteca real e seus links simbólicos correspondentes
            LIB_DIR="$(dirname "$lib")"
            LIB_NAME="$(basename "$lib")"
            # Extrai o prefixo base, ex.: libTKernel.so
            BASE_PREFIX=$(echo "$LIB_NAME" | sed -E 's/(\.so(\.[0-9]+)*)$/.so/')
            
            # Copia todas as versões e symlinks do mesmo toolkit
            for match in "$LIB_DIR"/$BASE_PREFIX*; do
                if [[ -e "$match" ]]; then
                    cp -a "$match" "$OUTPUT_DIR/lib/"
                    COPIED_COUNT=$((COPIED_COUNT + 1))
                fi
            done
        fi
    done
fi

# Se um diretório explícito de bibliotecas OCCT foi fornecido, copiar também dele
if [[ -n "$OCCT_LIB_DIR" && -d "$OCCT_LIB_DIR" ]]; then
    echo "==> Copiando bibliotecas adicionais de $OCCT_LIB_DIR..."
    for match in "$OCCT_LIB_DIR"/libTK*.so*; do
        if [[ -e "$match" ]]; then
            cp -a "$match" "$OUTPUT_DIR/lib/"
            COPIED_COUNT=$((COPIED_COUNT + 1))
        fi
    done
fi

echo "==> Bibliotecas dinâmicas copiadas: $COPIED_COUNT arquivos/links"

# Criar script inicializador portátil na raiz do pacote
cat <<'EOF' > "$OUTPUT_DIR/3dnator"
#!/usr/bin/env sh
# Inicializador portátil do 3dnator para Linux.
# Configura o carregamento das bibliotecas compartilhadas locais do OCCT.

set -e
BUNDLE_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
export LD_LIBRARY_PATH="$BUNDLE_DIR/lib:$BUNDLE_DIR/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$BUNDLE_DIR/bin/3dnator" "$@"
EOF
chmod 755 "$OUTPUT_DIR/3dnator"

# Copiar licenças e documentação essencial
for doc in "LICENSE" "THIRD_PARTY.md" "README.md"; do
    if [[ -f "$REPO_ROOT/$doc" ]]; then
        cp "$REPO_ROOT/$doc" "$OUTPUT_DIR/"
    fi
done

echo "==> Pacote portátil montado com sucesso em: $OUTPUT_DIR"

if [[ "$GENERATE_ARCHIVE" = true ]]; then
    ARCHIVE_PATH="${OUTPUT_DIR}.tar.gz"
    ARCHIVE_DIR="$(dirname "$OUTPUT_DIR")"
    ARCHIVE_NAME="$(basename "$OUTPUT_DIR")"
    echo "==> Compactando arquivo tarball: $ARCHIVE_PATH"
    tar -czf "$ARCHIVE_PATH" -C "$ARCHIVE_DIR" "$ARCHIVE_NAME"
    echo "==> Arquivo compactado com sucesso: $ARCHIVE_PATH"
fi

echo "==> Concluído!"
