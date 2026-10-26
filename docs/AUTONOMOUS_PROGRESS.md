# Execução autônoma

Autorização do usuário: continuar todas as etapas sem solicitar acompanhamento.

## Critério de trabalho

Concluir incrementos verificáveis, preservar decisões já aprovadas, executar os testes pertinentes e atualizar este registro. Não publicar o repositório, não substituir malhas por sólidos simulados e não declarar validações que não foram executadas.

## Ponto inicial

- 0.2.0: documento, comandos, persistência, recuperação, abas e corpos de malha; 22 testes aprovados.
- SDK OCCT ausente, DNS indisponível no terminal, `.git` somente leitura e janela interativa inacessível. Há captura real da interface sem janela.

## Sessão Codex (até 26/10/2026, 01h30)

Rascunhos 2D com solver, parâmetros e expressões, histórico de construção sobre a ponte OCCT (ainda sem compilar), montagem com subpeças, clones e grupos, folhas técnicas PNG/PDF. Ao fim da sessão: 39 testes aprovados, sem SDK OCCT.

## Sessão Claude Code (26/10/2026)

Executada em paralelo ao Codex até o encerramento dele. A partir daí, cada incremento foi registrado em commit próprio no repositório Git, criado nesta sessão.

1. OCCT 7.9.3 obtido sem privilégios (pacote oficial do Arch extraído em `target/`); corrigida a ordem dos componentes no CMake; primeira execução real da ponte, com `--validate` aprovado.
2. Referências topológicas estáveis: nomes de faces pela origem, regiões por entidades e arestas por pares de faces; formato `.3dnator` versão 7 com migração.
3. Ensaio `--validate-part` do critério de entrega da etapa 2 com o núcleo real.
4. Medições de leveza (`--benchmark`) na GPU integrada. Menos memória na visualização: linhas no shader, fusão exata de vértices e arestas vivas calculadas por geometria.
5. Seleção gráfica de arestas e faces pelos nomes de origem.
6. Etapa 3: furos (rebaixo, escareado, passantes, rosca registrada), padrões linear e circular, espelhamento, divisão, revolução por linha do rascunho e padrões no rascunho.
7. Etapa 4: padrões e espelhamento de instâncias e rascunho em face selecionada.
8. Etapa 5: corte A–A hachurado, revisão, data e notas de rosca nas folhas, ferramenta Medir com medidas exatas no B-rep.
9. Exportação STL com escopo, qualidade e resumo antes de gravar.

Ao fim: 86 testes aprovados com OCCT e 67 sem, clippy sem avisos nas duas configurações, ensaios de linha de comando aprovados.

## Pendências

Ver a seção Situação atual de [ROADMAP.md](../ROADMAP.md): Windows e Ubuntu (CI não executada), máquina de referência, interação real com a janela, controles visuais (isolamento, transparência, bloqueio, esmaecimento, perspectiva) e refinamentos de arredondamento, chanfro, casca, extrusão e folhas.
