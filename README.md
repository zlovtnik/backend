# AI Alibaba Cloud Test

Uma API web moderna construída com Elysia, Bun e PostgreSQL, com gerenciamento de conexões de banco de dados específico por usuário.

## Funcionalidades

- Limites de conexão de banco de dados por usuário
- Gerenciamento de organizações e usuários
- Controle de acesso baseado em permissões
- Endpoints RESTful
- Operações de banco de dados com tipagem segura usando Prisma
- Integração Contínua com GitHub Actions
- Verificação estrita de tipos TypeScript
- Containerização com Docker para fácil replicação
- Proxy reverso com Nginx para desenvolvimento e produção
- Gerenciamento de banco de dados com pgAdmin

## Documentação TypeScript

### Tipos de Dados

```typescript
/**
 * Representa um usuário no sistema
 * @interface User
 */
interface User {
  /** Identificador único do usuário */
  id: string;
  /** Email do usuário (deve ser único) */
  email: string;
  /** Nome do usuário */
  name: string;
  /** Senha criptografada do usuário */
  password: string;
  /** ID da organização à qual o usuário pertence */
  organizationId: string;
  /** Número máximo de conexões simultâneas permitidas */
  maxConnections: number;
  /** Data de criação do usuário */
  createdAt: Date;
  /** Data da última atualização */
  updatedAt: Date;
}

/**
 * Representa uma organização
 * @interface Organization
 */
interface Organization {
  /** Identificador único da organização */
  id: string;
  /** Nome da organização */
  name: string;
  /** Descrição opcional da organização */
  description?: string;
  /** Data de criação */
  createdAt: Date;
  /** Data da última atualização */
  updatedAt: Date;
}

/**
 * Representa uma permissão no sistema
 * @interface Permission
 */
interface Permission {
  /** Identificador único da permissão */
  id: string;
  /** Nome da permissão */
  name: string;
  /** Descrição opcional da permissão */
  description?: string;
  /** ID da organização à qual a permissão pertence */
  organizationId: string;
  /** Data de criação */
  createdAt: Date;
  /** Data da última atualização */
  updatedAt: Date;
}
```

### Repositórios

```typescript
/**
 * Interface para operações de banco de dados relacionadas a organizações
 * @interface OrganizationRepository
 */
interface OrganizationRepository {
  /**
   * Cria uma nova organização
   * @param data Dados da organização a ser criada
   * @returns Promise<Organization>
   */
  create(data: CreateOrganizationInput): Promise<Organization>;

  /**
   * Busca uma organização por ID
   * @param id ID da organização
   * @returns Promise<Organization | null>
   */
  findById(id: string): Promise<Organization | null>;

  /**
   * Lista todas as organizações
   * @returns Promise<Organization[]>
   */
  findAll(): Promise<Organization[]>;

  /**
   * Atualiza uma organização existente
   * @param id ID da organização
   * @param data Dados para atualização
   * @returns Promise<Organization>
   */
  update(id: string, data: UpdateOrganizationInput): Promise<Organization>;

  /**
   * Remove uma organização
   * @param id ID da organização
   * @returns Promise<Organization>
   */
  delete(id: string): Promise<Organization>;
}
```

### Endpoints da API

```typescript
/**
 * Configuração do servidor Elysia
 * @class App
 */
class App extends Elysia {
  /**
   * Endpoint de verificação de saúde
   * @route GET /api/health
   * @returns { status: 'ok' }
   */
  @get('/api/health')
  healthCheck() {
    return { status: 'ok' };
  }

  /**
   * Endpoint para listar organizações
   * @route GET /api/organizations
   * @param headers Headers da requisição (deve conter x-user-id)
   * @returns Promise<Organization[]>
   */
  @get('/api/organizations')
  async getOrganizations({ headers }: { headers: { 'x-user-id': string } }) {
    // Implementação
  }
}
```

## Pré-requisitos

- [Bun](https://bun.sh/) (v1.0.0 ou superior)
- Banco de dados PostgreSQL
- Node.js (para CLI do Prisma)
- [Docker](https://www.docker.com/) e [Docker Compose](https://docs.docker.com/compose/) (para containerização)
- [Nginx](https://nginx.org/) (para proxy reverso)

## Instalação

### Instalação Local

1. Clone o repositório
2. Instale as dependências:
```bash
bun install
```

3. Configure as variáveis de ambiente:
```bash
cp .env.example .env
```
Edite `.env` com suas credenciais:
```
DATABASE_URL="postgresql://user:password@localhost:5432/database"
PORT=3000
```

4. Execute as migrações do banco de dados:
```bash
bunx prisma migrate dev
```

### Instalação com Docker

1. Clone o repositório
2. Crie os diretórios necessários:
```bash
mkdir -p nginx/ssl frontend
```

3. Para desenvolvimento:
```bash
# Inicie todos os serviços
docker-compose up

# Acesse a aplicação
http://localhost
```

4. Para produção:
```bash
# Configure o ambiente para produção
export NGINX_CONF=production.conf

# Inicie todos os serviços
docker-compose up

# Acesse a aplicação
https://localhost
```

5. Para parar os containers:
```bash
docker-compose down
```

6. Para reiniciar os containers:
```bash
docker-compose restart
```

7. Para visualizar os logs:
```bash
docker-compose logs -f
```

### Configuração do Nginx

O projeto inclui duas configurações do Nginx:

1. **Desenvolvimento** (`nginx/development.conf`):
   - Proxy reverso para frontend Angular (porta 4200)
   - Proxy reverso para backend (porta 3000)
   - Sem SSL/TLS
   - Configurações básicas de proxy

2. **Produção** (`nginx/production.conf`):
   - SSL/TLS com HTTP/2
   - Headers de segurança
   - Compressão Gzip
   - Cache de arquivos estáticos
   - Redirecionamento HTTP para HTTPS
   - Timeouts configurados
   - Proteção contra acesso a arquivos ocultos

### Acesso ao pgAdmin

O pgAdmin está disponível em:
- URL: http://localhost:5050
- Email: admin@admin.com
- Senha: admin

Para conectar ao banco de dados no pgAdmin:
- Host: db
- Port: 5432
- Database: ai_alibaba_cloud
- Username: postgres
- Password: postgres

## Desenvolvimento

Inicie o servidor de desenvolvimento:
```bash
bun run dev
```

Execute os testes:
```bash
bun test
```

Execute a verificação de tipos:
```bash
bun run typecheck
```

## Gerenciamento de Conexões

O sistema implementa limites de conexão de banco de dados por usuário:

- Cada usuário possui um campo `maxConnections` no banco de dados
- O limite padrão é 1 conexão
- As conexões são gerenciadas e liberadas automaticamente
- O sistema impede exceder o limite de conexões do usuário

### Limites de Conexão por Usuário

- Usuários com `maxConnections = 1`: Uma conexão ativa
- Usuários com `maxConnections = 10`: Até 10 conexões ativas
- Os limites são aplicados por usuário

## Endpoints da API

Todos os endpoints requerem o header `x-user-id` para identificação do usuário.

### Verificação de Saúde
- `GET /api/health`
  - Retorna o status da aplicação
  - Não requer autenticação

### Organizações

- `GET /api/organizations`
  - Retorna estatísticas das organizações
  - Requer: header `x-user-id`

### Usuários

- `GET /api/users`
  - Retorna usuários por organização
  - Requer: header `x-user-id`

### Permissões

- `GET /api/permissions`
  - Retorna permissões do usuário
  - Requer: header `x-user-id`

## Integração Contínua

O projeto utiliza GitHub Actions para integração contínua:

### Workflow de Testes
- Executa em push para main e pull requests
- Configura banco de dados PostgreSQL para testes
- Executa migrações do banco de dados
- Executa suite de testes

### Workflow de Lint
- Executa em push para main e pull requests
- Realiza verificação de tipos
- Executa linting

### Workflow de Deploy
- Executa em push para main
- Configura banco de dados PostgreSQL de produção
- Constrói a aplicação
- Executa migrações do banco de dados
- Faz deploy para produção
- Verifica o deploy com health check

## Segurança de Tipos

O projeto implementa verificação estrita de tipos TypeScript:

- Todas as operações de banco de dados são type-safe
- Camada de repositório garante conversão adequada de tipos
- Endpoints da API possuem tipos corretos para requisições/respostas
- Tratamento adequado de null/undefined para campos opcionais

## Esquema do Banco de Dados

### Modelo de Usuário
```prisma
model User {
  id             String        @id @default(uuid())
  email          String        @unique
  name           String
  password       String
  organizationId String
  organization   Organization  @relation(fields: [organizationId], references: [id])
  permissions    Permission[]
  maxConnections Int          @default(1)
  createdAt      DateTime      @default(now())
  updatedAt      DateTime      @updatedAt
}
```

## Tratamento de Erros

A API retorna respostas de erro apropriadas:

- 400: ID de usuário ausente ou inválido
- 500: Erro interno do servidor
- Limite de conexão excedido: Erro quando usuário tenta exceder seu limite de conexões

## Licença

MIT