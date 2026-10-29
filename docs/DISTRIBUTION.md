# Empacotamento para Linux e Windows

Os scripts criam pacotes com o executável e as bibliotecas do OCCT. O workflow **Empacotar pré-release** (`.github/workflows/prerelease.yml`) compila a versão do Cargo no Linux e Windows, roda os testes com OCCT e prepara Flatpak e instalador EXE. A publicação no GitHub é deliberada, somente depois de conferir ambos os artefatos. Executar em ambiente limpo no mesmo Linux não demonstra portabilidade entre distribuições.

## Instalar a pré-release

Baixe os arquivos na [página de releases](https://github.com/trocolis/3dnator/releases) e confira o SHA-256 publicado. São pacotes experimentais, sem assinatura de código do aplicativo.

- **Linux x86_64:** instale o bundle com `flatpak install --user ./3dnator-0.3.0-alpha.1-linux-x86_64.flatpak`. Execute `flatpak run io.github.trocolis.ThreeDnator`. O runtime Freedesktop 25.08 vem do Flathub; o bundle informa essa origem. Se necessário, configure primeiro `flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo`. O aplicativo recebe acesso aos arquivos do hospedeiro para abrir/salvar projetos e exportações, além de GPU, Wayland e X11; não solicita rede.
- **Windows x64:** execute `3dnator-0.3.0-alpha.1-windows-x64-setup.exe`. O instalador solicita privilégios administrativos e inclui o redistribuível oficial Microsoft Visual C++ x64, com assinatura verificada durante a geração. OCCT e suas DLLs ficam adjacentes ao aplicativo. O executável do 3Dnator e o instalador ainda não têm certificado de assinatura próprio.

A versão exibida pelo aplicativo corresponde ao Cargo e à tag `v0.3.0-alpha.1`. O formato de projeto permanece 29. Pinturas por face/região são exportadas em STEP; a importação dessas cores no documento continua pendente.

## Gerar Flatpak e EXE

Execute manualmente **Empacotar pré-release** na revisão a distribuir; `platform` permite gerar ambos os formatos ou repetir somente Flatpak/Windows. O workflow não publica nem altera tags. O job Linux compila em Ubuntu 24.04 e verifica o carregamento/ensaios no **Platform** Freedesktop 25.08, sem depender do SDK; o job Windows valida a pasta relocada com PATH limpo e a instalação silenciosa do EXE. Cada artefato inclui seu SHA-256. A geração usa `scripts/collect_licenses.py` para textos e inventário das crates resolvidas (incluindo fontes) e `scripts/collect_native_licenses.py` para metadados/licenças dos pacotes do SDK. O inventário é mais amplo que os binários vinculados; `NATIVE_DEPENDENCIES.txt` distingue as bibliotecas efetivas.

Localmente, com Platform e Sdk Freedesktop 25.08 instalados e um executável compilado com ABI compatível:

```sh
python3 scripts/collect_licenses.py dist/release-licenses/rust
python3 scripts/collect_native_licenses.py dist/release-licenses/native
bash scripts/package_flatpak.sh target/release/nator-prototype dist/3dnator-0.3.0-alpha.1-linux-x86_64.flatpak 0.3.0-alpha.1 dist/release-licenses
```

O coletor nativo Linux usa `dpkg-query` e copyrights do Ubuntu; o Windows usa o prefixo e o cache conda. A geração recusa destinos existentes. A instalação local do Flatpak não é feita pelo script; os ensaios operam na árvore de build. A publicação só deve usar artefatos do mesmo commit da tag.

## Dependências e limites

A ponte C++ própria é estática; o OCCT permanece vinculado dinamicamente e pode ser substituído no pacote. Os scripts exigem um executável com OCCT, rejeitam bibliotecas não encontradas e gravam `NATIVE_DEPENDENCIES.txt`, distinguindo os arquivos incluídos dos requisitos fornecidos pelo sistema.

- Linux x86_64: inclui os SONAMEs OCCT efetivamente carregados e TBB quando o SDK o utiliza. Cada arquivo contém a biblioteca real, sem symlinks que apontem para a instalação de origem. O launcher resolve `lib/` relativamente à sua localização. glibc, runtime C++, bibliotecas de janela/fontes e drivers gráficos continuam sendo requisitos do host, conforme o manifesto. O pacote não é independente de todos os pacotes do sistema.
- Windows x64: usa `dumpbin /dependents` recursivamente, copiando as DLLs OCCT e as dependências do SDK encontradas junto ao executável, no diretório OCCT ou no PATH. Contratos API-set e DLLs existentes no diretório de sistema permanecem requisitos do Windows. O Microsoft Visual C++ Redistributable compatível pode ser necessário no destino. Não se copiam DLLs do sistema.

Bibliotecas carregadas dinamicamente por plugins ou drivers não são descobertas por essa análise de imports. A janela e os diálogos precisam de ensaio na máquina de destino.

Para alcançar uma base Linux mais antiga, compile o executável **e o SDK** nessa base. A CI usa Ubuntu 24.04 com o OCCT da distribuição; isso não comprova compatibilidade com Ubuntu 22.04 ou uma glibc anterior. O SDK 7.9.3 do Arch usado localmente não constitui base para prometer essa compatibilidade.

## Criar um pacote Linux

Compile com o SDK configurado conforme [PROTOTYPE.md](PROTOTYPE.md):

```sh
cargo build -p nator-prototype --release --features occt --locked
bash scripts/package_linux.sh --bin target/release/nator-prototype --archive
```

Se o SDK estiver fora dos caminhos do sistema, acrescente `--occt-lib-dir /caminho/para/sdk/lib`. Essa opção participa da resolução das dependências, em vez de apenas copiar arquivos depois da análise. `--output CAMINHO` escolhe outro destino.

Os caminhos informados são relativos ao diretório de invocação. O destino e o arquivo comprimido precisam ser novos; o script não remove nem sobrescreve arquivos existentes. O padrão é `dist/3dnator-linux-x86_64`, acompanhado de `.tar.gz` quando solicitado.

```sh
mv dist/3dnator-linux-x86_64 dist/3dnator-relocado
env -i PATH=/usr/bin:/bin ./dist/3dnator-relocado/3dnator --version
env -i PATH=/usr/bin:/bin ./dist/3dnator-relocado/3dnator --validate target/pacote-occt-001
env -i PATH=/usr/bin:/bin ./dist/3dnator-relocado/3dnator --validate-part target/pacote-peca-001
env -i PATH=/usr/bin:/bin ./dist/3dnator-relocado/3dnator --validate-step target/pacote-step-001
```

Cada ensaio também exige um diretório novo. As regressões dos scripts usam um SDK mínimo compilado em um caminho com espaços, com dependência transitiva TBB e symlinks; renomeiam o SDK antes de executar o pacote. Rode `python3 scripts/test_package_linux.py`.

## Criar um pacote Windows

Use PowerShell em um **Developer Command Prompt do Visual Studio**, para disponibilizar `dumpbin.exe`, e configure o SDK conforme [PROTOTYPE.md](PROTOTYPE.md):

```powershell
cargo build -p nator-prototype --release --features occt --locked
.\scripts\package_windows.ps1 -BinPath target\release\nator-prototype.exe -OcctBinDir C:\sdk\bin -Archive
```

O script também procura os layouts usuais de `OpenCASCADE_DIR`, conda e PATH. `-OutputDir` escolhe um destino novo. O padrão é `dist\3dnator-windows-x64`, acompanhado de `.zip` quando solicitado. Não há remoção ou sobrescrita de pacotes anteriores.

Teste o pacote após movê-lo, com PATH limitado a `%SystemRoot%\System32` e `%SystemRoot%`, executando `--version`, `--validate`, `--validate-part` e `--validate-step` e verificando cada código de saída. A CI prepara esses ensaios tanto no Windows quanto no Linux; os artefatos publicados pelo workflow são para validação, não releases deliberadas.

## Avisos e textos de licença

Os dois scripts incluem `LICENSE`, `THIRD_PARTY.md`, a documentação de uso do pacote e o diretório `licenses/`, com os textos integrais da LGPL 2.1 e da exceção OCCT obtidos do SDK 7.9.3. A procedência está em [licenses/README.md](../licenses/README.md).

[THIRD_PARTY.md](../THIRD_PARTY.md) identifica dependências e fontes originais. Esse resumo não é um inventário por versão de todas as crates nem de todas as variantes de SDK. Antes de uma release pública, revisar também avisos e textos das crates/fontes incorporadas e das dependências nativas efetivamente incluídas (por exemplo TBB, FreeType e zlib), conforme o manifesto do pacote. A inclusão dos textos OCCT não substitui essa revisão.
