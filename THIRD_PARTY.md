# Dependências de terceiros

O código próprio do 3dnator usa MIT. As dependências mantêm licenças independentes.

| Dependência direta | Uso | Origem |
| --- | --- | --- |
| egui/eframe | Interface e janela | https://github.com/emilk/egui |
| wgpu | Renderização e captura | https://github.com/gfx-rs/wgpu |
| serde/serde_json | Manifesto de projeto e relatórios de validação | https://github.com/serde-rs/serde e https://github.com/serde-rs/json |
| uuid | Identificação de projetos | https://github.com/uuid-rs/uuid |
| zip | Pacote autocontido `.3dnator` | https://github.com/zip-rs/zip2 |
| tempfile | Substituição segura do projeto salvo | https://github.com/Stebalien/tempfile |
| rfd | Diálogos nativos de arquivos | https://github.com/PolyMeilex/rfd |
| png | Captura de evidência | https://github.com/image-rs/image-png |
| pollster | Inicialização síncrona da captura | https://github.com/zesterer/pollster |
| OCCT 7.x, opcional e externo | Geometria CAD, projeções HLR (TKHLR) e intercâmbio XCAF/STEP (TKCDF, TKLCAF, TKCAF, TKXCAF e toolkit STEP da versão do SDK) | https://github.com/Open-Cascade-SAS/OCCT |

O OCCT é distribuído sob LGPL 2.1 com exceção adicional; a ponte própria não altera essa licença. O SDK não é incorporado ao repositório. Este inventário inicial não substitui os avisos e textos de licença das dependências transitivas: sua coleta precisa integrar o empacotamento antes da distribuição de binários.
