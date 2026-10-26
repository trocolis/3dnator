# Plano de implementação

Sequência aprovada de entregas. Este documento descreve trabalho planejado, não recursos implementados. As etapas não equivalem automaticamente a números de versão.

## Situação atual

Incremento 0.3.0, em 26/10/2026. As cinco etapas têm seus itens principais implementados e cobertos por testes e ensaios de linha de comando, com o núcleo OCCT 7.9.3 validado no Linux. O critério de entrega da etapa 2 é verificado por `--validate-part`. Detalhes em [docs/VALIDATION.md](docs/VALIDATION.md).

Pendências conhecidas, por etapa:

1. **Base técnica:** compilação, testes e validação OCCT no Windows e no Ubuntu (OCCT 7.6), já preparados na CI mas ainda não executados por falta de repositório remoto; medições na máquina de referência de 8 GB (as atuais foram feitas na GPU integrada de um notebook mais forte); interação real com teclado, mouse e diálogos, que os ensaios automáticos não cobrem.
2. **Primeira peça:** importação STEP sem cores nem estrutura de montagem.
3. **Construção mecânica:** hélice real de rosca (adiada por decisão de escopo); arredondamento de raio variável não faz parte do escopo inicial.
4. **Montagem:** espelhamento de peças que contêm subpeças; alinhamento contínuo por restrições não faz parte do escopo inicial.
5. **Documentação técnica:** folhas derivadas da malha de exibição (remoção aproximada de linhas ocultas, sem HLR exato do B-rep); cotas apenas gerais e não reposicionáveis; uma vista em corte por folha.

Interface: projeções ortográfica e em perspectiva disponíveis por aba; sombras opcionais ainda não foram implementadas.

## 1. Base técnica

- Avaliar e selecionar o núcleo CAD com casos representativos de sólidos, operações booleanas, arredondamentos e intercâmbio STEP.
- Montar janela e interface em egui/eframe, renderização em wgpu e navegação definida no README.
- Preparar compilação e validação em Linux e Windows, incluindo a integração do núcleo CAD; suporte a ambos faz parte dos critérios da base técnica.
- Usar 8 GB de RAM e GPU integrada como referência inicial de leveza. Medir tempo de abertura, memória em repouso e durante operações, resposta da navegação e recálculo, com cenários de peça simples, malha STL importada e instâncias repetidas. Registrar sistema, hardware, tamanho do modelo e configuração de tesselação; estabelecer limites numéricos após as primeiras medições.
- Separar as camadas e prever peças, definições compartilhadas, instâncias e transformações locais no modelo de dados desde o início.

## 2. Primeira peça utilizável

- Rascunhos com cotas, restrições e referências de desenho.
- Refinamento de rascunhos com aparar, estender, offset, geometria de construção e cotas de referência.
- Extrusão, histórico paramétrico e parâmetros com expressões.
- Desfazer/refazer, formato próprio de projeto e recuperação automática.
- Importação e exportação STEP; exportação STL para prototipação por impressão 3D e importação STL como corpo de malha para referência, sem conversão automática para sólido.
- Primeira peça de referência: suporte em L. Construir a forma básica primeiro e acrescentar furos e arredondamentos conforme as ferramentas forem implementadas.
- Critério de entrega: desenhar o suporte com medidas, extrudar, alterar uma cota com recálculo, salvar, reabrir e exportar para STEP e STL, verificando dimensões e integridade geométrica.

## 3. Construção mecânica

- Revolução e operações entre corpos, incluindo divisão.
- Furos e roscas simplificadas, chanfros, arredondamentos e casca.
- Padrões e espelhamento na construção e nos rascunhos.

## 4. Montagem

- Árvore hierárquica, instâncias, clones independentes e grupos.
- Posicionamento e alinhamento por geometria.
- Construção no contexto da montagem com referências independentes entre peças.
- Padrões e espelhamento de montagem.

## 5. Documentação técnica

- Medição completa e cotagem.
- Folhas associadas ao modelo, vistas e cortes.
- Exportação PNG e PDF vetorial, com estilos convencional e blueprint.

## Direção para o núcleo geométrico

A aplicação permanece em Rust; dependências C/C++ são permitidas para o núcleo CAD. OCCT foi integrado por uma ponte C++ própria e validado no Linux (OCCT 7.9.3): extrusão, revolução, booleanas, chanfro, arredondamento, casca, tesselação, ida e volta STEP/STL, gestão de erros e de propriedade de memória. Truck permanece como alternativa em Rust, sem avaliação.

Antes de consolidar a dependência para distribuição, falta verificar no Windows, preservar cores e montagem na importação/exportação STEP, avaliar o empacotamento das bibliotecas compartilhadas e reunir os avisos de licença (LGPL 2.1 com exceção). Medir tempo e memória na máquina de referência, sem presumir leveza pelo idioma de implementação.
