-- NFe 4.00 Schema - Brazilian Electronic Invoice System
-- This migration creates the complete schema for NFe (Nota Fiscal Eletrônica) 4.00 compliance

-- ===========================================
-- CORE NFe TABLES
-- ===========================================

-- NFe documents (main invoice table)
CREATE TABLE nfe_documents (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(36) NOT NULL,
    nfe_id VARCHAR(50) UNIQUE NOT NULL, -- NFe access key
    serie VARCHAR(3) NOT NULL,
    numero VARCHAR(9) NOT NULL,
    modelo VARCHAR(2) NOT NULL DEFAULT '55', -- 55=NF-e, 65=NFC-e
    versao VARCHAR(4) NOT NULL DEFAULT '4.00',
    status VARCHAR(20) NOT NULL DEFAULT 'draft', -- draft, authorized, cancelled, denied
    tipo_operacao VARCHAR(1) NOT NULL DEFAULT '1', -- 0=entry, 1=exit
    tipo_emissao VARCHAR(1) NOT NULL DEFAULT '1', -- 1=normal, 2=contingency, etc.
    finalidade VARCHAR(1) NOT NULL DEFAULT '1', -- 1=normal, 2=complement, 3=adjustment, 4=return
    indicador_presencial VARCHAR(1) NOT NULL DEFAULT '9', -- 0=not applicable, 1=presential, etc.

    -- Dates
    data_emissao TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    data_saida_entrada TIMESTAMP WITH TIME ZONE,
    data_autorizacao TIMESTAMP WITH TIME ZONE,
    data_cancelamento TIMESTAMP WITH TIME ZONE,

    -- Values
    valor_total DECIMAL(15,2) NOT NULL DEFAULT 0,
    valor_desconto DECIMAL(15,2) DEFAULT 0,
    valor_frete DECIMAL(15,2) DEFAULT 0,
    valor_seguro DECIMAL(15,2) DEFAULT 0,
    valor_outras_despesas DECIMAL(15,2) DEFAULT 0,
    valor_produtos DECIMAL(15,2) NOT NULL DEFAULT 0,
    valor_impostos DECIMAL(15,2) NOT NULL DEFAULT 0,

    -- References
    pedido_compra VARCHAR(60),
    contrato VARCHAR(60),

    -- Additional info
    informacoes_adicionais TEXT,
    informacoes_fisco TEXT,

    -- Protocol and authorization
    protocolo_autorizacao VARCHAR(50),
    motivo_cancelamento TEXT,
    justificativa_contingencia TEXT,

    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(tenant_id, serie, numero),
    CHECK (valor_total >= 0),
    CHECK (valor_produtos >= 0),
    CHECK (valor_impostos >= 0)
);

-- Emitters (issuers of NFe)
CREATE TABLE nfe_emitters (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(36) NOT NULL,
    cnpj VARCHAR(14) NOT NULL,
    cpf VARCHAR(11),
    razao_social VARCHAR(120) NOT NULL,
    nome_fantasia VARCHAR(60),
    inscricao_estadual VARCHAR(14),
    inscricao_estadual_subst_tributario VARCHAR(14),
    inscricao_municipal VARCHAR(15),
    cnae VARCHAR(7),
    regime_tributario VARCHAR(1) NOT NULL DEFAULT '1', -- 1=simple national, 2=presumed profit, 3=real profit

    -- Address
    logradouro VARCHAR(125),
    numero VARCHAR(10),
    complemento VARCHAR(60),
    bairro VARCHAR(60),
    codigo_municipio VARCHAR(7),
    municipio VARCHAR(60),
    uf VARCHAR(2),
    cep VARCHAR(8),
    codigo_pais VARCHAR(4) DEFAULT '1058',
    pais VARCHAR(60) DEFAULT 'Brasil',
    telefone VARCHAR(14),

    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(tenant_id, cnpj),
    CHECK (cnpj IS NOT NULL OR cpf IS NOT NULL),
    CHECK (LENGTH(cnpj) = 14 OR LENGTH(cpf) = 11)
);

-- Recipients (customers/clients)
CREATE TABLE nfe_recipients (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(36) NOT NULL,
    tipo_pessoa VARCHAR(1) NOT NULL DEFAULT 'J', -- F=individual, J=company
    cnpj VARCHAR(14),
    cpf VARCHAR(11),
    id_estrangeiro VARCHAR(20),
    razao_social VARCHAR(120) NOT NULL,
    nome_fantasia VARCHAR(60),
    inscricao_estadual VARCHAR(14),
    inscricao_municipal VARCHAR(15),
    inscricao_suframa VARCHAR(9),
    email VARCHAR(60),

    -- Address
    logradouro VARCHAR(125),
    numero VARCHAR(10),
    complemento VARCHAR(60),
    bairro VARCHAR(60),
    codigo_municipio VARCHAR(7),
    municipio VARCHAR(60),
    uf VARCHAR(2),
    cep VARCHAR(8),
    codigo_pais VARCHAR(4) DEFAULT '1058',
    pais VARCHAR(60) DEFAULT 'Brasil',
    telefone VARCHAR(14),

    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(tenant_id, cnpj),
    UNIQUE(tenant_id, cpf),
    CHECK (tipo_pessoa IN ('F', 'J')),
    CHECK ((tipo_pessoa = 'J' AND cnpj IS NOT NULL) OR (tipo_pessoa = 'F' AND cpf IS NOT NULL) OR id_estrangeiro IS NOT NULL)
);

-- ===========================================
-- PRODUCT AND ITEM TABLES
-- ===========================================

-- Products catalog
CREATE TABLE nfe_products (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(36) NOT NULL,
    codigo VARCHAR(60) NOT NULL,
    ean VARCHAR(14),
    descricao VARCHAR(120) NOT NULL,
    ncm VARCHAR(8),
    cfop VARCHAR(4),
    unidade VARCHAR(6) NOT NULL DEFAULT 'UN',
    valor_unitario DECIMAL(15,2) NOT NULL DEFAULT 0,
    valor_frete DECIMAL(15,2) DEFAULT 0,
    valor_seguro DECIMAL(15,2) DEFAULT 0,
    valor_desconto DECIMAL(15,2) DEFAULT 0,
    valor_outras_despesas DECIMAL(15,2) DEFAULT 0,

    -- Tax information
    icms_cst VARCHAR(3),
    icms_aliquota DECIMAL(5,2),
    ipi_cst VARCHAR(3),
    ipi_aliquota DECIMAL(5,2),
    pis_cst VARCHAR(3),
    pis_aliquota DECIMAL(5,2),
    cofins_cst VARCHAR(3),
    cofins_aliquota DECIMAL(5,2),

    -- Additional info
    informacoes_adicionais TEXT,

    -- Metadata
    ativo BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(tenant_id, codigo),
    CHECK (valor_unitario >= 0)
);

-- NFe items (products in invoices)
CREATE TABLE nfe_items (
    id SERIAL PRIMARY KEY,
    nfe_document_id INTEGER NOT NULL REFERENCES nfe_documents(id) ON DELETE CASCADE,
    numero_item INTEGER NOT NULL,
    product_id INTEGER REFERENCES nfe_products(id),

    -- Product info
    codigo VARCHAR(60) NOT NULL,
    ean VARCHAR(14),
    descricao VARCHAR(120) NOT NULL,
    ncm VARCHAR(8),
    cfop VARCHAR(4) NOT NULL,
    unidade VARCHAR(6) NOT NULL DEFAULT 'UN',
    quantidade DECIMAL(15,4) NOT NULL DEFAULT 1,
    valor_unitario DECIMAL(21,10) NOT NULL,
    valor_total DECIMAL(15,2) NOT NULL,

    -- Discounts and additions
    valor_desconto DECIMAL(15,2) DEFAULT 0,
    valor_frete DECIMAL(15,2) DEFAULT 0,
    valor_seguro DECIMAL(15,2) DEFAULT 0,
    valor_outras_despesas DECIMAL(15,2) DEFAULT 0,

    -- Tax calculations
    valor_bc_icms DECIMAL(15,2),
    valor_icms DECIMAL(15,2),
    valor_bc_icms_st DECIMAL(15,2),
    valor_icms_st DECIMAL(15,2),
    valor_bc_ipi DECIMAL(15,2),
    valor_ipi DECIMAL(15,2),
    valor_bc_pis DECIMAL(15,2),
    valor_pis DECIMAL(15,2),
    valor_bc_cofins DECIMAL(15,2),
    valor_cofins DECIMAL(15,2),

    -- Additional info
    informacoes_adicionais TEXT,
    numero_pedido_compra VARCHAR(15),
    item_pedido_compra VARCHAR(6),

    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(nfe_document_id, numero_item),
    CHECK (quantidade > 0),
    CHECK (valor_unitario >= 0),
    CHECK (valor_total >= 0)
);

-- ===========================================
-- TAX TABLES
-- ===========================================

-- ICMS tax details
CREATE TABLE nfe_icms (
    id SERIAL PRIMARY KEY,
    nfe_item_id INTEGER NOT NULL REFERENCES nfe_items(id) ON DELETE CASCADE,
    cst VARCHAR(3) NOT NULL,
    modalidade_bc VARCHAR(1),
    valor_bc DECIMAL(15,2),
    aliquota DECIMAL(5,2),
    valor DECIMAL(15,2),
    modalidade_bc_st VARCHAR(1),
    percentual_mva_st DECIMAL(5,2),
    percentual_reducao_bc_st DECIMAL(5,2),
    valor_bc_st DECIMAL(15,2),
    aliquota_st DECIMAL(5,2),
    valor_st DECIMAL(15,2),
    percentual_reducao_bc_efetiva DECIMAL(5,2),
    valor_bc_efetiva DECIMAL(15,2),
    aliquota_efetiva DECIMAL(5,2),
    valor_efetivo DECIMAL(15,2),

    -- Additional ICMS info
    codigo_beneficio_fiscal VARCHAR(10),
    percentual_diferimento DECIMAL(5,2),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- IPI tax details
CREATE TABLE nfe_ipi (
    id SERIAL PRIMARY KEY,
    nfe_item_id INTEGER NOT NULL REFERENCES nfe_items(id) ON DELETE CASCADE,
    cst VARCHAR(3) NOT NULL,
    classe_enquadramento VARCHAR(5),
    cnpj_produtor VARCHAR(14),
    codigo_selo_controle VARCHAR(6),
    quantidade_selo INTEGER,
    modalidade_bc VARCHAR(1),
    valor_bc DECIMAL(15,2),
    aliquota DECIMAL(5,2),
    quantidade_unidade DECIMAL(16,4),
    valor_unidade DECIMAL(15,4),
    valor DECIMAL(15,2),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- PIS tax details
CREATE TABLE nfe_pis (
    id SERIAL PRIMARY KEY,
    nfe_item_id INTEGER NOT NULL REFERENCES nfe_items(id) ON DELETE CASCADE,
    cst VARCHAR(3) NOT NULL,
    modalidade_bc VARCHAR(1),
    valor_bc DECIMAL(15,2),
    aliquota_percentual DECIMAL(5,2),
    aliquota_valor DECIMAL(15,4),
    quantidade_vendida DECIMAL(16,4),
    valor DECIMAL(15,2),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- COFINS tax details
CREATE TABLE nfe_cofins (
    id SERIAL PRIMARY KEY,
    nfe_item_id INTEGER NOT NULL REFERENCES nfe_items(id) ON DELETE CASCADE,
    cst VARCHAR(3) NOT NULL,
    modalidade_bc VARCHAR(1),
    valor_bc DECIMAL(15,2),
    aliquota_percentual DECIMAL(5,2),
    aliquota_valor DECIMAL(15,4),
    quantidade_vendida DECIMAL(16,4),
    valor DECIMAL(15,2),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- ===========================================
-- TRANSPORT AND LOGISTICS
-- ===========================================

-- Transport information
CREATE TABLE nfe_transport (
    id SERIAL PRIMARY KEY,
    nfe_document_id INTEGER NOT NULL REFERENCES nfe_documents(id) ON DELETE CASCADE,

    modalidade_frete VARCHAR(1) NOT NULL DEFAULT '9', -- 0=by issuer, 1=by recipient, 2=by third party, 9=without freight

    -- Carrier info
    cnpj VARCHAR(14),
    cpf VARCHAR(11),
    razao_social VARCHAR(60),
    inscricao_estadual VARCHAR(14),
    endereco_completo VARCHAR(200),
    municipio VARCHAR(60),
    uf VARCHAR(2),

    -- Vehicle info
    placa_veiculo VARCHAR(8),
    uf_veiculo VARCHAR(2),
    rntc VARCHAR(20),

    -- Freight values
    valor_servico DECIMAL(15,2),
    valor_bc_retencao_icms DECIMAL(15,2),
    valor_icms_retido DECIMAL(15,2),
    cfop VARCHAR(4),
    codigo_municipio VARCHAR(7),

    -- Additional info
    informacoes_fisco TEXT,

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Transport volumes
CREATE TABLE nfe_transport_volumes (
    id SERIAL PRIMARY KEY,
    nfe_transport_id INTEGER NOT NULL REFERENCES nfe_transport(id) ON DELETE CASCADE,
    quantidade INTEGER NOT NULL,
    especie VARCHAR(60),
    marca VARCHAR(60),
    numeracao VARCHAR(60),
    peso_liquido DECIMAL(15,3),
    peso_bruto DECIMAL(15,3),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- ===========================================
-- PAYMENT AND FINANCE
-- ===========================================

-- Payment methods
CREATE TABLE nfe_payments (
    id SERIAL PRIMARY KEY,
    nfe_document_id INTEGER NOT NULL REFERENCES nfe_documents(id) ON DELETE CASCADE,
    indicador_pagamento VARCHAR(1) NOT NULL DEFAULT '0', -- 0=payment, 1=change
    forma_pagamento VARCHAR(2) NOT NULL, -- 01=money, 02=check, 03=credit card, etc.
    valor DECIMAL(15,2) NOT NULL,

    -- Card payment details (if applicable)
    tipo_integracao VARCHAR(1), -- 1=integrated, 2=non-integrated
    cnpj_credenciadora VARCHAR(14),
    bandeira VARCHAR(20),
    numero_autorizacao VARCHAR(20),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- ===========================================
-- ADDITIONAL INFORMATION AND REFERENCES
-- ===========================================

-- NFe references (for corrections, complements, etc.)
CREATE TABLE nfe_references (
    id SERIAL PRIMARY KEY,
    nfe_document_id INTEGER NOT NULL REFERENCES nfe_documents(id) ON DELETE CASCADE,
    tipo VARCHAR(1) NOT NULL, -- 0=NFe, 1=NF, 2=CTe, 3=Cupom Fiscal, etc.
    chave_acesso VARCHAR(44),
    uf VARCHAR(2),
    mes_ano VARCHAR(4),
    cnpj VARCHAR(14),
    modelo VARCHAR(2),
    serie VARCHAR(3),
    numero VARCHAR(9),

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Additional fiscal information
CREATE TABLE nfe_fiscal_info (
    id SERIAL PRIMARY KEY,
    nfe_document_id INTEGER NOT NULL REFERENCES nfe_documents(id) ON DELETE CASCADE,
    campo VARCHAR(20) NOT NULL,
    texto TEXT NOT NULL,

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(nfe_document_id, campo)
);

-- ===========================================
-- INDEXES FOR PERFORMANCE
-- ===========================================

-- Core NFe document indexes
CREATE INDEX idx_nfe_documents_tenant_status ON nfe_documents(tenant_id, status);
CREATE INDEX idx_nfe_documents_emissao ON nfe_documents(data_emissao);
CREATE INDEX idx_nfe_documents_chave ON nfe_documents(nfe_id);

-- Product indexes
CREATE INDEX idx_nfe_products_tenant ON nfe_products(tenant_id);
CREATE INDEX idx_nfe_products_codigo ON nfe_products(tenant_id, codigo);

-- Item indexes
CREATE INDEX idx_nfe_items_document ON nfe_items(nfe_document_id);
CREATE INDEX idx_nfe_items_product ON nfe_items(product_id);

-- Tax indexes
CREATE INDEX idx_nfe_icms_item ON nfe_icms(nfe_item_id);
CREATE INDEX idx_nfe_ipi_item ON nfe_ipi(nfe_item_id);
CREATE INDEX idx_nfe_pis_item ON nfe_pis(nfe_item_id);
CREATE INDEX idx_nfe_cofins_item ON nfe_cofins(nfe_item_id);

-- Transport indexes
CREATE INDEX idx_nfe_transport_document ON nfe_transport(nfe_document_id);

-- Payment indexes
CREATE INDEX idx_nfe_payments_document ON nfe_payments(nfe_document_id);

-- Reference indexes
CREATE INDEX idx_nfe_references_document ON nfe_references(nfe_document_id);

-- Fiscal info indexes
CREATE INDEX idx_nfe_fiscal_info_document ON nfe_fiscal_info(nfe_document_id);

-- Partial unique indexes for conditional constraints
CREATE UNIQUE INDEX idx_nfe_recipients_tenant_cnpj ON nfe_recipients(tenant_id, cnpj) WHERE cnpj IS NOT NULL;
CREATE UNIQUE INDEX idx_nfe_recipients_tenant_cpf ON nfe_recipients(tenant_id, cpf) WHERE cpf IS NOT NULL;
