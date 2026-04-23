#!/bin/bash

# Configurações do repositório e artefato
REPO="nic-wq/nedit"
BINARY_NAME="nedit_linux"
INSTALL_PATH="/usr/local/bin/nedit"

# Inicializa variável para decidir se aceita pre-releases
UNSTABLE=false

# 1. Processar argumentos (flags)
for arg in "$@"; do
    case $arg in
        --unstable)
            UNSTABLE=true
            shift
            ;;
    esac
done

echo "Buscando versão mais recente..."

# 2. Definir a URL da API do GitHub
# Se unstable for true, pegamos a primeira release da lista geral (pode ser pre-release)
# Caso contrário, usamos o endpoint 'latest' que garante ser uma release estável
if [ "$UNSTABLE" = true ]; then
    echo "Modo UNSTABLE ativado: Incluindo pre-releases na busca."
    API_URL="https://api.github.com/repos/$REPO/releases"
    # No endpoint de lista, pegamos o primeiro item que contenha o binário desejado
    DOWNLOAD_URL=$(curl -s "$API_URL" | \
                   grep "browser_download_url" | \
                   grep "$BINARY_NAME" | \
                   head -n 1 | \
                   cut -d '"' -f 4)
else
    API_URL="https://api.github.com/repos/$REPO/releases/latest"
    DOWNLOAD_URL=$(curl -s "$API_URL" | \
                   grep "browser_download_url" | \
                   grep "$BINARY_NAME" | \
                   cut -d '"' -f 4)
fi

# Valida se a URL foi extraída corretamente
if [ -z "$DOWNLOAD_URL" ]; then
    echo "Erro: Não foi possível localizar o arquivo $BINARY_NAME no repositório $REPO."
    exit 1
fi

# 3. Baixar o binário
echo "Baixando de: $DOWNLOAD_URL"
curl -L -o "$BINARY_NAME" "$DOWNLOAD_URL"

if [ $? -ne 0 ]; then
    echo "Erro durante o download."
    exit 1
fi

# 4. Definir permissões de execução
chmod +x "$BINARY_NAME"

# 5. Mover o binário para o diretório de destino (requer privilégios de superusuário)
echo "Instalando em $INSTALL_PATH..."
sudo mv "$BINARY_NAME" "$INSTALL_PATH"

# 6. Verificação final
if [ $? -eq 0 ]; then
    echo "Instalação concluída com sucesso."
else
    echo "Erro durante a instalação em $INSTALL_PATH."
    exit 1
fi