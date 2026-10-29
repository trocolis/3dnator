# Executar o 3dnator

Pré-release experimental 0.3.0-alpha.1. O 3dnator usa Open CASCADE Technology (OCCT) para construir sólidos CAD. O código próprio usa a licença MIT; avisos e fontes das dependências estão em `THIRD_PARTY.md`, e os textos OCCT em `licenses/OCCT/`.

No Flatpak, execute `flatpak run io.github.trocolis.ThreeDnator`; no Windows, use o instalador EXE. Nos pacotes portáteis de validação, siga as instruções abaixo.

No Linux, execute `./3dnator` na raiz do pacote. No Windows, execute `3dnator.exe`. Mantenha as bibliotecas junto ao aplicativo: `lib/` no Linux, DLLs adjacentes ao executável no Windows. É possível mover a pasta inteira.

O sistema precisa fornecer drivers gráficos compatíveis, bibliotecas de janela e os runtimes indicados em `NATIVE_DEPENDENCIES.txt`. No Windows pode ser necessário instalar o Microsoft Visual C++ Redistributable. Este pacote ainda não comprova compatibilidade com todas as distribuições Linux ou versões do Windows.

- Scroll: zoom no cursor; botão do meio e arraste: deslocar a vista; Shift com botão do meio e arraste: orbitar.
- Ctrl+S: salvar o projeto `.3dnator`; Ctrl+Z: desfazer; Ctrl+Shift+Z ou Ctrl+Y: refazer.
- STEP permite intercâmbio de sólidos e montagens; STL permanece uma malha de referência ou de impressão. Guarde o projeto para preservar o histórico paramétrico.

Para verificar o carregamento sem abrir a janela, execute `./3dnator --version` (Linux) ou `.\3dnator.exe --version` (PowerShell). `--help` lista os ensaios CLI; cada um requer um diretório de saída novo.

As bibliotecas do OCCT permanecem dinâmicas e podem ser substituídas por versões compatíveis. O código-fonte original está em <https://github.com/Open-Cascade-SAS/OCCT>.
