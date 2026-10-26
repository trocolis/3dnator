# Plano de implementação

Sequência aprovada de entregas. Este documento descreve trabalho planejado, não recursos implementados. As etapas não equivalem automaticamente a números de versão.

## Situação atual

Incremento 0.2.0: documento com peças, múltiplos corpos de malha e instâncias, posicionamento numérico, comandos atômicos com desfazer/refazer, formato `.3dnator`, recuperação automática e interface em abas. A base de persistência da etapa 2 está implementada; rascunhos e operações paramétricas ainda não. A ponte OCCT permanece pendente de SDK e os workflows Windows ainda não foram executados. Consulte [docs/PROTOTYPE.md](docs/PROTOTYPE.md).

Próximo incremento funcional: rascunhos 2D e suas cotas/restrições, preservando a separação entre documento, comandos, solução geométrica e interface. A validação nativa OCCT continua necessária antes de confirmar o fluxo de sólidos e STEP.

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

A aplicação permanece em Rust; dependências C/C++ são permitidas para o núcleo CAD. OCCT é o candidato prioritário para validação, com Truck como alternativa em Rust. Ainda não há núcleo integrado ou validado.

Antes de confirmar a integração, verificar extrusão/revolução, booleanas, chanfro/arredondamento, casca, tesselação, importação/exportação STEP com cores e montagem, gestão de erros e memória, compilação/distribuição e licença. Medir tempo e memória em casos representativos, sem presumir leveza pelo idioma de implementação.
