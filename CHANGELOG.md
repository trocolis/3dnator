# Histórico de mudanças

## Não lançado — 0.2.0 em desenvolvimento

- Documento com peças, múltiplos corpos de malha, instâncias e transformações locais separadas da montagem.
- Comandos atômicos, desfazer/refazer por projeto e identificação de alterações desde o último salvamento; malhas compartilhadas entre snapshots.
- Pacote `.3dnator` versão interna 1, autocontido, com manifesto JSON e malhas f64; gravação temporária validada antes da substituição do arquivo principal.
- Abas independentes, árvore de peças/corpos, importação STL conforme contexto, renomeação, visibilidade, posicionamento numérico e exclusão de corpos.
- Diálogos nativos, Ctrl+S, exportação de corpo STL e confirmação de fechamento com Salvar/Descartar/Cancelar.
- Recuperação automática separada, com opção de restaurar cópias ao abrir o aplicativo e marcador que impede remover projetos normais na limpeza.
- Preparação de cena e buffers CPU em segundo plano, proteção contra resultados de outra aba/revisão e captura da interface real sem janela.
- Ensaio CLI do ciclo importar/salvar/reabrir sem o STL original, recuperação e exportação; CI estendida para esse fluxo.

## 0.1.0 — protótipo técnico anterior

Nenhuma versão foi publicada ou marcada com tag nesta sessão.

### Implementado no protótipo

- Workspace Rust com geometria própria, fronteira de núcleo CAD, intercâmbio, renderizador e aplicação experimental.
- Suporte em L de referência com volume e dimensões conhecidos.
- Leitura STL ASCII/binário com escala e limites de recursos; exportação binária com verificações de geometria e precisão, sem sobrescrita silenciosa.
- Visualizador egui/eframe/wgpu em português, com iluminação simples, câmera ortográfica, deslocamento, órbita, zoom no cursor e vistas predefinidas.
- Captura real do renderizador e nove testes de geometria, STL e navegação.
- Licença MIT para o código próprio, inventário inicial de dependências e documentação de execução e validação.

### Preparado para validação

- Ponte C++ opcional para OCCT 7.x, isolada por API Rust própria, com gestão de propriedade e conversão de exceções.
- Ensaios de sólidos, corte, chanfro, arredondamento, tesselação e ida e volta STEP/STL, com relatório de tempo e memória.
- Workflows de compilação/testes Rust e validação OCCT em Linux e Windows. Execução com SDK OCCT e execução Windows ainda pendentes.

### Planejado para o editor

- Projeto autocontido `.3dnator`, recuperação automática, abas, peças, corpos, instâncias, clones, grupos e montagem.
- Rascunhos com restrições, cotas, referências, diagnóstico, reparo, aparar, estender, offset e geometria de construção.
- Histórico paramétrico, parâmetros/expressões, operações atômicas e desfazer/refazer.
- Extrusão, revolução, booleanas, refinamentos, casca, furos, roscas simplificadas, padrões e espelhamento.
- Importação STEP como peças e STL como corpos de malha, referências independentes entre peças e exportação STL por corpo.
- Medição, cotagem, cortes, folhas técnicas associadas ao modelo e exportação PNG/PDF com opção blueprint.
- Linux e Windows como plataformas iniciais; referência de desempenho com 8 GB de RAM e GPU integrada.

Escopo e decisões completos em README.md, ARCHITECTURE.md e ROADMAP.md. Recursos planejados não são funcionalidades entregues neste protótipo.
