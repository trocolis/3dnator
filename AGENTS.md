# Convenções de desenvolvimento

## Linguagem e comunicação
- Escrever código Rust com nomes de variáveis, funções, tipos, módulos e arquivos em inglês.
- Escrever comentários e documentação explicativa em português. Comentar abundantemente intenções, regras geométricas, unidades, decisões e casos delicados; manter os comentários atualizados e evitar apenas repetir o código.
- Usar português como idioma padrão da interface. Centralizar textos em recursos de tradução com identificadores estáveis, permitindo adicionar idiomas sem alterar a lógica de negócio.
- Escrever mensagens de commit descritivas em português, explicando a mudança e sua finalidade.

## Arquitetura e qualidade
- Manter separadas as camadas de geometria, solução de rascunhos, núcleo CAD, documento e histórico, persistência, comandos e interface.
- Impedir que regras de negócio dependam de egui, eframe ou wgpu. Encapsular bibliotecas geométricas atrás de interfaces próprias.
- A aplicação deve ser em Rust, mas o núcleo CAD pode usar dependências C/C++ quando justificadas por confiabilidade e cobertura de recursos. Isolar a integração nativa, conversão de erros e gestão de memória; não expor tipos da biblioteca externa às demais camadas.
- Usar comandos para alterações do documento, oferecendo base consistente para desfazer/refazer e histórico paramétrico.
- Preferir funções coesas, responsabilidades claras, composição e dependências explícitas. Evitar abstrações sem necessidade demonstrada.
- Representar unidades, identificadores e transformações com tipos claros; distinguir coordenadas locais e de montagem. Usar milímetros e radianos internamente, com conversões na interface.
- Tratar erros com Result e mensagens compreensíveis; evitar panic, unwrap e expect em caminhos de produção. Validar entradas numéricas e arquivos importados.
- Manter cálculos demorados fora da thread da interface e aplicar resultados de forma consistente com a revisão do documento.
- Suportar Linux e Windows desde a base. Evitar caminhos e comportamentos específicos de um sistema fora dos módulos de integração; verificar compilação e operações representativas em ambos, incluindo dependências nativas.
- Testar regras geométricas, recálculo paramétrico, transformações, persistência e regressões relevantes. Executar cargo fmt, cargo clippy e testes pertinentes quando houver código Rust.
- Preservar trabalho existente do usuário e não versionar segredos, arquivos temporários ou artefatos de compilação.

## Git e versões
- O código próprio do projeto usa a licença MIT, conforme LICENSE. Preservar licenças e atribuições das dependências e recursos de terceiros.
- Versionar continuamente em commits pequenos e coerentes, após verificar a alteração. Não criar commits artificiais por intervalo de tempo.
- Usar versões X.Y.Z: X para grandes releases; Y para alterações ou adições de funcionalidades; Z para pequenos ajustes e correções.
- Manter registro das mudanças em CHANGELOG.md e sincronizar a versão com os metadados do aplicativo quando existirem.
- Usar tags vX.Y.Z para releases deliberadas. Um commit não exige uma nova release.
- Não publicar nem enviar alterações a repositórios remotos sem autorização.
