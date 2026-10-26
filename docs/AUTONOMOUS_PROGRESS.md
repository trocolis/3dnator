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
10. Arredondamento e chanfro de várias arestas, chanfro por ângulo, casca com várias faces e para fora; extrusão até uma face e com resultado combinado.
11. Controles visuais: isolamento, transparência, bloqueio de seleção e esmaecimento na construção.

Ao fim: 90 testes aprovados com OCCT e 68 sem, clippy sem avisos nas duas configurações, ensaios de linha de comando aprovados.

## Pendências

Ver a seção Situação atual de [ROADMAP.md](../ROADMAP.md): Windows e Ubuntu (CI não executada), máquina de referência, interação real com a janela, sombras, importação STEP com cores e montagem e folhas com HLR exato.

## Continuação Codex (26/10/2026)

- Revisados código, histórico e roadmap; estado inicial limpo, com os 68 testes sem OCCT aprovados.
- Implementada a projeção em perspectiva por aba, mantendo a ortográfica como padrão, com raios de seleção, medições, movimento livre e navegação coerentes com a matriz da GPU. A vista isométrica conserva escala, alvo e projeção; a orientação escolhida no cubo é guardada na câmera da aba.
- Acrescentados cinco testes de câmera e ampliados os dois existentes para ambas as projeções: recálculo dos limites da cena, zoom no cursor, seleção, redução aparente com a distância, planos de recorte, enquadramento e movimento no plano do alvo.
- Capturas reais ortográfica, perspectiva e da interface em `target/codex-validation/`, pelo wgpu/llvmpipe, sem erros de validação. Triangulação e transparência verificadas nas duas projeções.
- Formatação e Clippy sem avisos aprovados; 73 testes sem OCCT e 95 com OCCT aprovados. Guia do protótipo atualizado para remover descrições obsoletas da versão 0.2.0.

## Continuação Codex — conjuntos espelhados (26/10/2026)

- Removida a limitação de espelhar peças com subpeças: hierarquias aninhadas com definições repetidas geram uma cópia independente, mantendo o compartilhamento interno e compensando a reflexão nas posições rígidas de cada ocorrência.
- Preservados grupos internos durante a clonagem e o espelhamento, e a visibilidade de ocorrências e corpos finais ocultos. Grupos do nível externo não são copiados para dentro do conjunto.
- Corrigida a extração dos ângulos de matrizes perto de ±90°, usando `atan2` com a projeção do eixo em vez de `asin`; normais de planos são convertidas sem subtrair translações grandes.
- Testes cobrem reflexão em plano oblíquo no referencial de um pai rotacionado, compartilhamento por vários caminhos da hierarquia, grupos internos, visibilidade, salvamento/reabertura, duplo espelhamento, desfazer/refazer e falhas atômicas.
- Ensaio com OCCT verifica conjuntos mistos de malhas e sólidos paramétricos, propriedades exatas e alteração independente de parâmetros das subpeças após reabrir o projeto.
- Formatação e Clippy aprovados nas duas configurações; 78 testes sem OCCT e 101 com OCCT aprovados. O formato de projeto continua na versão interna 7, sem migração nova.
