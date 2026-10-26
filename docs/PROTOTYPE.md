# Protótipo 0.2.0 — projetos com corpos de malha

Este incremento entrega a base de documento e persistência da etapa 2. Rascunhos, restrições e operações paramétricas ainda não estão implementados. O histórico mostrado na interface corresponde às edições da sessão, não à construção paramétrica de um sólido.

## Abrir a aplicação

Requer Rust 1.95 ou superior, compilador C/C++ para dependências e bibliotecas de janela/driver gráfico. A validação local usa Rust 1.99.1 em Linux x86_64.

```sh
cargo run -p nator-prototype --locked
```

A janela começa com um projeto vazio. Use **Inserir suporte em L** ou **Importar STL** para criar uma peça com um corpo. Na árvore, selecione a peça/corpo e use **Construir nesta peça**: importações seguintes acrescentam corpos à definição ativa. **Voltar à montagem** faz a próxima importação criar outra peça.

A árvore permite selecionar, mostrar/ocultar peças e corpos. As propriedades oferecem renomeação, posicionamento X/Y/Z em milímetros, rotação em graus e exclusão de corpos. Posições de corpos são locais à peça; posições de instâncias são relativas à montagem. Corpos só podem ser editados na peça ativa em construção; instâncias são posicionadas na montagem. A seleção gráfica diretamente na área 3D ainda não existe.

Projetos abrem em abas. Cada aba mantém documento, seleção, câmera, contexto e sequência de desfazer/refazer próprios. Há um único conjunto de buffers GPU para a aba ativa. A representação da cena ainda combina malhas das instâncias, sem instanciamento gráfico otimizado.

## Navegação e comandos

- Scroll aproxima/afasta em direção ao cursor.
- Botão do meio arrasta a vista; Shift com botão do meio orbita.
- Botões oferecem vistas frontal, superior, lateral e isométrica, além de enquadramento.
- X/Y/Z acompanham a câmera; o cubo de navegação completo ainda está pendente.
- Ctrl+S salva; Ctrl+Z desfaz; Ctrl+Shift+Z ou Ctrl+Y refazem. Campos de texto conservam seu próprio desfazer.
- As operações concluídas são atômicas e a sequência guarda até 100 ações, compartilhando as malhas entre snapshots.
- Ao fechar uma aba modificada ou encerrar a aplicação, o editor oferece Salvar, Descartar ou Cancelar.

A composição da cena e a preparação CPU dos buffers ocorrem em segundo plano. A interface só faz o upload para a GPU. Enquanto uma tarefa estiver ativa, as edições ficam desabilitadas; navegação e troca de abas continuam disponíveis. Resultados de importação e cena identificam a aba e a revisão que os originaram.

## Salvar e recuperar

**Salvar** e **Salvar como…** usam diálogos nativos. O arquivo `.3dnator` contém as definições, corpos de malha, instâncias, nomes, visibilidade e transformações. O STL de origem pode ser movido ou apagado após o salvamento. A versão interna inicial é 1, independente da versão do aplicativo.

A gravação temporária é validada antes de substituir o projeto anterior. A recuperação é gravada separadamente, após dois segundos de inatividade e disponibilidade da thread de trabalho, e é oferecida ao iniciar a aplicação. Uma cópia recuperada precisa ser salva explicitamente. `NATOR_RECOVERY_DIR` permite escolher o diretório das cópias.

Detalhes e limites estão em [PROJECT_FORMAT.md](PROJECT_FORMAT.md). Miniaturas, câmera persistida, sólidos CAD, rascunhos e histórico paramétrico ainda não fazem parte do arquivo. Desfazer/refazer começa vazio ao reabrir.

## Importar e exportar STL

A importação aceita STL ASCII e binário, com até 128 MiB e dois milhões de triângulos; a escala para milímetros é escolhida antes de importar. Malhas abertas são aceitas como referência. Vértices não finitos, triângulos degenerados e arquivos incompletos são rejeitados, sem reparo automático.

Selecione um corpo na árvore e use **Exportar corpo STL**. O arquivo usa coordenadas locais à peça, incluindo a transformação do corpo e excluindo a posição da montagem. O nome sugerido combina peça e corpo. O destino deve ser novo; exportações ainda não substituem arquivos existentes.

O STL binário verifica a quantização f32, fechamento local de arestas, orientação e volume positivo. Não detecta todos os defeitos de impressão, como auto-interseções ou cascas sobrepostas. Exportação em lote e presets de tesselação CAD ainda estão pendentes. O projeto atual armazena as malhas importadas, sem convertê-las em sólidos.

## Ensaios sem janela

```sh
cargo run -p nator-prototype --no-default-features --locked -- --validate-project target/project-check/run-001
cargo run -p nator-prototype --no-default-features --locked -- --inspect-project target/project-check/run-001/support.3dnator
```

O diretório deve ser novo. O ensaio cria uma peça com dois corpos, aplica posições locais/de montagem, salva o projeto, exercita desfazer/refazer, grava recuperação, remove o STL original, reabre e exporta um corpo. Gera `report.json`, `support.3dnator`, `recovery.3dnator` e `body.stl`.

A CLI anterior de STL continua disponível:

```sh
cargo run -p nator-prototype --no-default-features --locked -- --export-reference reference.stl
cargo run -p nator-prototype --no-default-features --locked -- --inspect reference.stl
```

Capturas reais usando o mesmo renderizador/composição da aplicação, sem servidor gráfico:

```sh
cargo run -p nator-prototype --locked -- --render-preview target/project-check/solid.png
cargo run -p nator-prototype --locked -- --render-project-preview target/project-check/run-001/support.3dnator target/project-check/editor.png
```

Os PNGs devem ter destinos novos. A captura da interface carrega o projeto, seleciona seu primeiro corpo e acrescenta uma aba auxiliar vazia apenas para a visualização. Não modifica o projeto de origem nem grava recuperação. Essas capturas não substituem o teste interativo de teclado, mouse ou diálogos do sistema.

## Medições de leveza

```sh
cargo run --release -p nator-prototype --features occt -- --benchmark target/bench/run-001
NATOR_BENCH_GPU=high cargo run --release -p nator-prototype --features occt -- --benchmark target/bench/run-002
```

Mede abertura de projeto, preparação da cena, envio à GPU, tempo de quadro em órbita contínua e memória (RSS atual e pico, no Linux) nos cenários de peça simples, malha STL importada de 1 milhão de triângulos e 400 instâncias, além do recálculo paramétrico quando o OCCT está habilitado. Cada cenário roda em um processo filho, para que o pico de memória seja por cenário. A GPU integrada é preferida por padrão; `NATOR_BENCH_GPU=high` escolhe a dedicada. O relatório inclui CPU, memória total, adaptador, driver e perfil de compilação. Resultados em [VALIDATION.md](VALIDATION.md).

## OCCT e STEP

O SDK OCCT continua opcional e externo. A ponte usa CMake e bibliotecas compartilhadas OCCT 7.x (alvo 7.6 a 7.9), com API Rust própria. Configure `OpenCASCADE_DIR` para a pasta de `OpenCASCADEConfig.cmake`, ou `CMAKE_PREFIX_PATH` para o prefixo de instalação, além do caminho de bibliotecas do sistema. No Windows, a pasta dos DLLs precisa estar no PATH.

A ponte foi compilada e validada no Linux com OCCT 7.9.3 (pacote `opencascade` do Arch). Em Arch Linux, `sudo pacman -S opencascade` instala um SDK compatível. Sem acesso de administrador, o pacote oficial pode ser extraído em uma pasta local e indicado por `OpenCASCADE_DIR` (`<prefixo>/usr/lib/cmake/opencascade`) e `LD_LIBRARY_PATH` (`<prefixo>/usr/lib`), desde que as dependências do pacote já estejam instaladas no sistema.

A receita prevista para Ubuntu 24.04 usa `cmake`, `libocct-data-exchange-dev` e `libocct-modeling-algorithms-dev` (OCCT 7.6). O workflow Windows usa conda-forge com `occt>=7.8,<8`, CMake e MSVC. Essas duas receitas ainda não foram executadas.

```sh
cargo run -p nator-prototype --no-default-features --features occt --locked -- --validate validation-output/occt-run-001
cargo run -p nator-prototype --no-default-features --features occt --locked -- --validate-part validation-output/part-001
cargo test -p nator-kernel -p nator-commands --features occt --locked
```

`--validate` verifica volumes, cortes, chanfro/arredondamento e ida e volta STEP/STL com primitivas. `--validate-part` executa o critério de entrega da etapa 2 com comandos do documento: suporte em L cotado por parâmetros, extrusão, furo, arredondamento, alteração das cotas com recálculo, desfazer/refazer, salvar, reabrir e exportar STEP e STL, conferindo volume analítico, dimensões e topologia da malha. Os testes com `--features occt` cobrem a nomeação estável de faces e as referências do histórico.

A importação STEP lê geometria, sem preservar cores ou hierarquia. Um sólido STEP importado entra no documento com B-rep e faces nomeadas pela posição, estáveis porque essa geometria não é recalculada.

Sem o SDK e a feature `occt`, CAD/STEP falham explicitamente; não há núcleo simulado.

## Verificações e plataformas

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Resultados locais e limitações estão em [VALIDATION.md](VALIDATION.md). A CI inclui Rust e o ciclo de projeto em Linux/Windows, além dos jobs OCCT; os workflows estão preparados e ainda não foram executados remotamente. Não foi possível testar os diálogos nativos nem a janela interativa por falta de acesso ao compositor/servidor X. A captura real de interface usa renderização por software.

## Dependências

O código próprio usa MIT; as dependências mantêm suas licenças, listadas inicialmente em [THIRD_PARTY.md](../THIRD_PARTY.md). O SDK OCCT não está incorporado ao repositório. Os avisos/textos das dependências ainda precisam integrar o empacotamento de binários.

O lock mantém `clipboard-win` 5.4.1, marcado como yanked no índice do cache offline. Essa cadeia deve ser atualizada e validada antes de distribuir Windows. O terminal continua sem acesso de rede para obter novas versões ou instalar OCCT.
