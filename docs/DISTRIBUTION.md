# Estratégia de empacotamento e distribuição

Este documento descreve os critérios, a arquitetura e os procedimentos para distribuição do **3dnator** em Linux e Windows, com ênfase na integração das dependências nativas do OpenCASCADE Technology (OCCT) e na conformidade com suas licenças.

---

## 1. Princípios de distribuição

O 3dnator prioriza a experiência do usuário final, oferecendo pacotes portáteis autocontidos que não exigem privilégios de administrador ou instalação global de pacotes complexos do sistema para serem executados:

- **Autocontido e portátil:** O pacote inclui o binário da aplicação e as bibliotecas compartilhadas nativas necessárias para execução direta.
- **Isolamento de dependências:** O executável não interfere nas versões de bibliotecas instaladas no sistema hospedeiro.
- **Conformidade estrita de licenças:** A distribuição atende rigorosamente aos termos da licença LGPL 2.1 e da *Open CASCADE Technology Exception* (versão 1.0).

---

## 2. Dependências nativas e resolução dinâmica

O núcleo CAD do 3dnator (`nator-kernel`) vincula-se à biblioteca C++ do OpenCASCADE (OCCT 7.x) através de uma ponte nativa estática (`nator_occt`), que por sua vez se vincula dinamicamente às bibliotecas compartilhadas oficiais do OCCT:

### Toolkits OCCT utilizados

- **Fundação e Matemática:** `TKernel`, `TKMath`
- **Geometria e Topologia:** `TKG2d`, `TKG3d`, `TKGeomBase`, `TKBRep`, `TKGeomAlgo`, `TKTopAlgo`
- **Operações de Modelagem:** `TKPrim`, `TKBO`, `TKBool`, `TKFillet`, `TKOffset`, `TKMesh`
- **Projeções e Desenho:** `TKHLR`
- **Estrutura CAF/XCAF e STEP:** `TKXSBase`, `TKCDF`, `TKLCAF`, `TKCAF`, `TKVCAF`, `TKService`, `TKV3d`, `TKDE`, `TKXCAF`, `TKDESTEP` (ou `TKSTEP`, `TKSTEPBase`, `TKSTEPAttr`, `TKXDESTEP` em versões anteriores a 7.8).

---

## 3. Estratégia por plataforma

### 3.1. Linux (x86_64)

No Linux, a resolução de bibliotecas compartilhadas ELF respeita a seguinte hierarquia:

1. **Estrutura do pacote:**
   ```text
   3dnator-linux-x86_64/
   ├── 3dnator             # Script inicializador executável
   ├── bin/
   │   └── 3dnator         # Binário compilado ELF
   ├── lib/
   │   ├── libTKernel.so.7.9
   │   ├── libTKMath.so.7.9
   │   └── ... (links e arquivos .so do OCCT)
   ├── LICENSE
   ├── THIRD_PARTY.md
   └── README.md
   ```

2. **Launcher portátil (`3dnator`):**
   O script na raiz do pacote resolve seu próprio diretório de forma confiável e define a variável `LD_LIBRARY_PATH` apontando para o subdiretório `lib/` antes de invocar o binário real:
   ```sh
   #!/usr/bin/env sh
   set -e
   BUNDLE_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
   export LD_LIBRARY_PATH="$BUNDLE_DIR/lib:$BUNDLE_DIR/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
   exec "$BUNDLE_DIR/bin/3dnator" "$@"
   ```

3. **Compatibilidade de glibc:**
   Para ampla portabilidade entre distribuições Linux (Ubuntu, Debian, Fedora, Arch), o binário de release para distribuição deve ser compilado em uma imagem de sistema com versão razoavelmente antiga da glibc (por exemplo Ubuntu 22.04 LTS ou imagem manylinux/alma8 na esteira de CI), garantindo que funcione nas distribuições modernas sem incompatibilidade de símbolos da biblioteca C.

### 3.2. Windows (x64)

No Windows, a política de busca de DLLs do sistema operacional prioriza automaticamente o diretório onde o executável está localizado:

1. **Estrutura do pacote:**
   ```text
   3dnator-windows-x64/
   ├── 3dnator.exe         # Binário compilado da aplicação
   ├── TKernel.dll
   ├── TKMath.dll
   ├── TKBRep.dll
   ├── ... (demais DLLs do OCCT)
   ├── LICENSE
   ├── THIRD_PARTY.md
   └── README.md
   ```

2. **Resolução de dependências:**
   Ao colocar as DLLs do OCCT lado a lado com `3dnator.exe`, o carregador do Windows resolve todas as dependências nativas sem necessidade de manipulação de variáveis de ambiente do sistema (`PATH`).
   
3. **Runtime MSVC:**
   O pacote requer o Microsoft Visual C++ Redistributable (arquivos `vcruntime140.dll` e `msvcp140.dll`), comumente já presentes em instalações modernas do Windows ou instaláveis através do redistribuível oficial da Microsoft.

---

## 4. Conformidade com a LGPL 2.1 e Exceção OCCT

O código próprio do 3dnator é licenciado sob os termos da licença **MIT**. A utilização do OpenCASCADE Technology respeita integralmente as seguintes exigências legais:

1. **Vinculação dinâmica:**
   O OCCT é consumido exclusivamente por meio de vinculação dinâmica (`.so` no Linux, `.dll` no Windows). Dessa forma, o usuário mantém a liberdade expressa na LGPL 2.1 de inspecionar, recompilar ou substituir qualquer biblioteca do OCCT por uma versão personalizada sem violar a licença do projeto ou exigir o fornecimento de código proprietário fechado.

2. **Aviso proeminente de uso:**
   Conforme estipulado pela *Open CASCADE Technology Exception (version 1.0)*, a documentação de apoio do projeto e os arquivos distribuídos (`THIRD_PARTY.md`, `README.md`) contêm aviso explícito e proeminente informando que o software utiliza e baseia-se em recursos fornecidos pelo Open CASCADE Technology.

3. **Disponibilização de fontes e licenças:**
   O texto integral da LGPL versão 2.1 e da exceção do OCCT acompanham a distribuição, acompanhados de indicações e links para os repositórios oficiais onde o código-fonte original do OpenCASCADE pode ser obtido.

---

## 5. Procedimentos de geração de pacotes

### 5.1. No Linux

1. **Compilar o binário em modo release:**
   ```sh
   export OpenCASCADE_DIR=/caminho/para/o/sdk/opencascade
   export LD_LIBRARY_PATH="$OpenCASCADE_DIR/lib:$LD_LIBRARY_PATH"
   cargo build -p nator-prototype --release --features occt --locked
   ```

2. **Executar o script de empacotamento:**
   ```sh
   ./scripts/package_linux.sh --bin target/release/nator-prototype --archive
   ```
   O pacote portátil descompactado será criado em `dist/3dnator-linux-x86_64` e o arquivo comprimido em `dist/3dnator-linux-x86_64.tar.gz`.

3. **Testar a execução do pacote distribuído:**
   ```sh
   ./dist/3dnator-linux-x86_64/3dnator --version
   ./dist/3dnator-linux-x86_64/3dnator --help
   ```

### 5.2. No Windows (PowerShell)

1. **Compilar o binário em modo release:**
   ```powershell
   $env:OpenCASCADE_DIR = "C:\caminho\para\opencascade"
   cargo build -p nator-prototype --release --features occt --locked
   ```

2. **Executar o script de empacotamento:**
   ```powershell
   .\scripts\package_windows.ps1 -BinPath "target\release\nator-prototype.exe" -Archive
   ```
   O diretório portátil será gerado em `dist\3dnator-windows-x64` e o arquivo compactado em `dist\3dnator-windows-x64.zip`.
