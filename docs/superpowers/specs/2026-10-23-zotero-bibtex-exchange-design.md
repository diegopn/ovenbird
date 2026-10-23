# Troca de arquivos BibTeX com a Zotero

## Objetivo

Permitir que a pessoa transfira referências entre a Zotero e o Ovenbird usando arquivos `.bib`, sem conectar contas ao Ovenbird. A troca será manual; não será uma sincronização automática entre contas.

## O que esta etapa deve fazer

- Na tela de Referências, oferecer um atalho para importar um arquivo BibTeX exportado da Zotero.
- Manter a exportação BibTeX do Ovenbird disponível para que a pessoa possa importar o arquivo na Zotero.
- Incorporar as referências importadas à biblioteca local, preservando os dados existentes e ignorando chaves de citação duplicadas, como o importador atual já faz.
- Manter a biblioteca local e os arquivos exportados no formato BibTeX padrão.

## O que não faz parte desta etapa

- Login na Zotero, chave de acesso à API, acesso à rede, vínculo de conta ou sincronização automática.
- Transferência de anexos, notas, coleções ou outros dados da Zotero que o BibTeX não representa.
- Garantia de que importar o mesmo arquivo novamente atualize os itens existentes. Um arquivo exportado não é uma cópia completa da biblioteca Zotero.
- Sincronização automática nos dois sentidos. Uma etapa futura precisará guardar identificadores estáveis dos itens remotos e lidar com alterações conflitantes.

## Caminhos considerados

1. **Troca manual por BibTeX — escolhido para a primeira etapa.** Usar a exportação e a importação BibTeX da Zotero e os recursos BibTeX que o Ovenbird já tem. Isso evita credenciais de conta e um serviço mantido pelo projeto, além de manter os dados portáteis. A pessoa precisa mover o arquivo manualmente, e o formato tem os limites descritos acima.
2. **API da Zotero.** A autorização pelo navegador pode permitir buscar referências e, no futuro, enviá-las de volta. Exige registrar o aplicativo, guardar credenciais sensíveis e lidar com identificadores remotos, permissões e conflitos. É mais do que a primeira etapa de transferência manual precisa.
3. **GitHub ou Google Drive como armazenamento.** Esses serviços podem guardar ou sincronizar um arquivo, mas acrescentam autenticação e possíveis conflitos entre arquivos sem melhorar o formato bibliográfico. Não são uma opção melhor para a primeira etapa.

## Proposta

Adicionar um atalho de troca com a Zotero à barra superior, visível somente na tela de Referências, ao lado do botão de minimizar. Ao ativá-lo, a pessoa encontrará as ações existentes de importar e exportar BibTeX, com nomes que expliquem o uso com a Zotero. A importação continuará usando a janela segura de seleção de arquivos do sistema: o app lê o `.bib`, incorpora as referências à biblioteca local e informa quantas foram adicionadas. A exportação salva a biblioteca local em `.bib` para a pessoa importar na Zotero.

Assim, a biblioteca `.bib` local continua sendo editável e exportável, e as duas direções de transferência manual ficam acessíveis. Isso não é sincronização contínua. A documentação da Zotero confirma a importação e a exportação BibTeX e alerta que exportar e importar não transfere uma biblioteca inteira como cópia exata: BibTeX não leva anexos e coleções, e alguns dados podem ser alterados na reimportação. Consulte [importação de BibTeX](https://www.zotero.org/support/kb/importing_standardized_formats), [exportação da Zotero](https://www.zotero.org/support/kb/exporting) e [limites e mapeamento de campos](https://www.zotero.org/support/kb/field_mappings).

## Restrições e decisões de implementação

- Reutilizar as janelas de seleção de arquivos, as ações de importação e exportação, a mesclagem da biblioteca local e o leitor BibTeX existentes; não adicionar uma dependência.
- Manter as permissões do Flatpak como estão e usar o fluxo atual de seleção de arquivos pelo sistema.
- Atualizar as mensagens em inglês e manter os catálogos de português e espanhol sincronizados.
- A troca manual não exige novas credenciais nem arquivos de metadados persistentes.
