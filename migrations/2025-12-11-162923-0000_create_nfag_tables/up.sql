-- Create NFAg main table
CREATE TABLE nfag (
  id SERIAL PRIMARY KEY,
  chave VARCHAR(44) UNIQUE NOT NULL,
  tenant_id VARCHAR(36) NOT NULL REFERENCES tenants(id),
  versao VARCHAR(4),
  xml_content TEXT,
  status VARCHAR(20) DEFAULT 'pending',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create NFAg identification table
CREATE TABLE nfag_ide (
  id SERIAL PRIMARY KEY,
  nfag_id INTEGER NOT NULL REFERENCES nfag(id) ON DELETE CASCADE,
  cUF INTEGER NOT NULL,
  tpAmb INTEGER NOT NULL,
  mod INTEGER NOT NULL,
  serie INTEGER NOT NULL,
  nNF BIGINT NOT NULL,
  cNF VARCHAR(8) NOT NULL,
  cDV VARCHAR(1) NOT NULL,
  dhEmi TIMESTAMP WITH TIME ZONE NOT NULL,
  tpEmis INTEGER NOT NULL,
  nSiteAutoriz INTEGER NOT NULL,
  cMunFG INTEGER NOT NULL,
  finNFAg INTEGER NOT NULL,
  tpFat INTEGER NOT NULL,
  verProc VARCHAR(20) NOT NULL,
  dhCont TIMESTAMP WITH TIME ZONE,
  xJust TEXT,
  UNIQUE(nfag_id)
);

-- Create NFAg emitter table
CREATE TABLE nfag_emit (
  id SERIAL PRIMARY KEY,
  nfag_id INTEGER NOT NULL REFERENCES nfag(id) ON DELETE CASCADE,

  -- Tax identification
  CNPJ VARCHAR(14) NOT NULL, -- CNPJ of the emitter (required for legal entities)
  IE VARCHAR(14), -- State tax registration (optional)

  -- Company information
  xNome VARCHAR(60) NOT NULL, -- Legal company name
  xFant VARCHAR(60), -- Trade name/fantasy name (optional)

  -- Contact information
  email VARCHAR(60), -- Contact email (optional)
  telefone VARCHAR(14), -- Contact phone (optional)

  -- Address information
  logradouro VARCHAR(125), -- Street address
  numero VARCHAR(10), -- Street number
  complemento VARCHAR(60), -- Address complement (optional)
  bairro VARCHAR(60), -- Neighborhood/district
  codigo_municipio VARCHAR(7), -- IBGE municipality code
  municipio VARCHAR(60), -- Municipality name
  UF VARCHAR(2), -- State code (e.g., 'SP', 'RJ')
  CEP VARCHAR(8), -- Postal code (8 digits, no hyphen)
  codigo_pais VARCHAR(4) DEFAULT '1058', -- Country code (1058 = Brazil)
  pais VARCHAR(60) DEFAULT 'Brasil', -- Country name

  -- Emitter classification
  tipo_emitente VARCHAR(1) DEFAULT '1', -- 1=normal emitter, 2=substitute emitter (optional)

  -- Audit timestamps
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

  -- Constraints
  UNIQUE(nfag_id),
  CHECK (CNPJ IS NOT NULL), -- CNPJ is always required for emitters
  CHECK (LENGTH(CNPJ) = 14), -- CNPJ must be exactly 14 digits
  CHECK (IE IS NULL OR LENGTH(IE) >= 8), -- IE should be valid if provided (minimum 8 digits)
  CHECK (codigo_municipio IS NULL OR LENGTH(codigo_municipio) = 7), -- IBGE code is 7 digits
  CHECK (CEP IS NULL OR LENGTH(CEP) = 8), -- CEP is 8 digits
  CHECK (UF IS NULL OR LENGTH(UF) = 2), -- State code is 2 characters
  CHECK (tipo_emitente IN ('1', '2')) -- Valid emitter types
);

-- Create NFAg destination table
CREATE TABLE nfag_dest (
  id SERIAL PRIMARY KEY,
  nfag_id INTEGER NOT NULL REFERENCES nfag(id) ON DELETE CASCADE,

  -- Tax identification (exactly one must be provided)
  CNPJ VARCHAR(14), -- CNPJ for legal entities
  CPF VARCHAR(11), -- CPF for individuals
  idEstrangeiro VARCHAR(20), -- Foreign ID for non-Brazilian entities

  -- Company/Person information
  xNome VARCHAR(60), -- Legal name (required when no tax ID provided)

  -- Additional tax information
  IE VARCHAR(14), -- State tax registration (optional)
  IM VARCHAR(15), -- Municipal tax registration (optional)

  -- Contact information
  email VARCHAR(60), -- Contact email (optional)
  telefone VARCHAR(14), -- Contact phone (optional)

  -- Address information (optional for destinations)
  logradouro VARCHAR(125), -- Street address
  numero VARCHAR(10), -- Street number
  complemento VARCHAR(60), -- Address complement (optional)
  bairro VARCHAR(60), -- Neighborhood/district
  codigo_municipio VARCHAR(7), -- IBGE municipality code
  municipio VARCHAR(60), -- Municipality name
  UF VARCHAR(2), -- State code (e.g., 'SP', 'RJ')
  CEP VARCHAR(8), -- Postal code (8 digits, no hyphen)
  codigo_pais VARCHAR(4) DEFAULT '1058', -- Country code (1058 = Brazil)
  pais VARCHAR(60) DEFAULT 'Brasil', -- Country name

  -- Audit timestamps
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

  -- Constraints
  CHECK ((CNPJ IS NOT NULL) OR (CPF IS NOT NULL) OR (idEstrangeiro IS NOT NULL)), -- At least one tax ID required
  CHECK (xNome IS NOT NULL), -- Legal name always required
  CHECK (CNPJ IS NULL OR LENGTH(CNPJ) = 14), -- CNPJ must be exactly 14 digits if provided
  CHECK (CPF IS NULL OR LENGTH(CPF) = 11), -- CPF must be exactly 11 digits if provided
  CHECK (IE IS NULL OR LENGTH(IE) >= 8), -- IE should be valid if provided (minimum 8 digits)
  CHECK (codigo_municipio IS NULL OR LENGTH(codigo_municipio) = 7), -- IBGE code is 7 digits
  CHECK (CEP IS NULL OR LENGTH(CEP) = 8), -- CEP is 8 digits
  CHECK (UF IS NULL OR LENGTH(UF) = 2) -- State code is 2 characters
);

-- Create NFAg total table
CREATE TABLE nfag_total (
  id SERIAL PRIMARY KEY,
  nfag_id INTEGER NOT NULL REFERENCES nfag(id) ON DELETE CASCADE,
  vBC DECIMAL(15,2),
  vICMS DECIMAL(15,2),
  vICMSDeson DECIMAL(15,2),
  vFCPUFDest DECIMAL(15,2),
  vICMSUFDest DECIMAL(15,2),
  vICMSUFRemet DECIMAL(15,2),
  vFCP DECIMAL(15,2),
  vBCST DECIMAL(15,2),
  vST DECIMAL(15,2),
  vFCPST DECIMAL(15,2),
  vFCPSTRet DECIMAL(15,2),
  vProd DECIMAL(15,2),
  vFrete DECIMAL(15,2),
  vSeg DECIMAL(15,2),
  vDesc DECIMAL(15,2),
  vII DECIMAL(15,2),
  vIPI DECIMAL(15,2),
  vIPIDevol DECIMAL(15,2),
  vPIS DECIMAL(15,2),
  vCOFINS DECIMAL(15,2),
  vOutro DECIMAL(15,2),
  vNF DECIMAL(15,2),
  vTotTrib DECIMAL(15,2),
  UNIQUE(nfag_id)
);

-- Create consultation situation table
CREATE TABLE cons_sit_nfag (
  id SERIAL PRIMARY KEY,
  tenant_id VARCHAR(36) NOT NULL REFERENCES tenants(id),
  tpAmb INTEGER NOT NULL,
  xServ VARCHAR(20) NOT NULL,
  chNFAg VARCHAR(44) NOT NULL,
  versao VARCHAR(4),
  xml_request TEXT,
  xml_response TEXT,
  cStat INTEGER,
  xMotivo TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create consultation service status table
CREATE TABLE cons_stat_serv_nfag (
  id SERIAL PRIMARY KEY,
  tenant_id VARCHAR(36) NOT NULL REFERENCES tenants(id),
  tpAmb INTEGER NOT NULL,
  xServ VARCHAR(20) NOT NULL,
  versao VARCHAR(4),
  xml_request TEXT,
  xml_response TEXT,
  cStat INTEGER,
  xMotivo TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create events table
CREATE TABLE evento_nfag (
  id SERIAL PRIMARY KEY,
  tenant_id VARCHAR(36) NOT NULL REFERENCES tenants(id),
  chNFAg VARCHAR(44) NOT NULL,
  tpEvento INTEGER NOT NULL,
  nSeqEvento INTEGER NOT NULL,
  versao VARCHAR(4),
  xml_content TEXT,
  status VARCHAR(20) DEFAULT 'pending',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create return table
CREATE TABLE ret_nfag (
  id SERIAL PRIMARY KEY,
  tenant_id VARCHAR(36) NOT NULL REFERENCES tenants(id),
  tpAmb INTEGER NOT NULL,
  cStat INTEGER NOT NULL,
  xMotivo TEXT,
  versao VARCHAR(4),
  xml_content TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_nfag_tenant_id ON nfag(tenant_id);
CREATE INDEX idx_nfag_chave ON nfag(chave);
CREATE INDEX idx_nfag_status ON nfag(status);
-- Composite indexes for tenant-scoped queries
CREATE INDEX idx_nfag_tenant_chave ON nfag(tenant_id, chave);
CREATE INDEX idx_nfag_tenant_status ON nfag(tenant_id, status);
CREATE INDEX idx_nfag_ide_nfag_id ON nfag_ide(nfag_id);
CREATE INDEX idx_nfag_emit_nfag_id ON nfag_emit(nfag_id);
CREATE INDEX idx_nfag_dest_nfag_id ON nfag_dest(nfag_id);
CREATE INDEX idx_nfag_total_nfag_id ON nfag_total(nfag_id);
CREATE INDEX idx_cons_sit_tenant_id ON cons_sit_nfag(tenant_id);
CREATE INDEX idx_cons_sit_chNFAg ON cons_sit_nfag(chNFAg);
CREATE INDEX idx_cons_stat_tenant_id ON cons_stat_serv_nfag(tenant_id);
CREATE INDEX idx_evento_tenant_id ON evento_nfag(tenant_id);
CREATE INDEX idx_evento_chNFAg ON evento_nfag(chNFAg);
CREATE INDEX idx_ret_tenant_id ON ret_nfag(tenant_id);

-- Trigger function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Add triggers to consultation tables
CREATE TRIGGER update_cons_sit_nfag_updated_at BEFORE UPDATE ON cons_sit_nfag
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_cons_stat_serv_nfag_updated_at BEFORE UPDATE ON cons_stat_serv_nfag
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_evento_nfag_updated_at BEFORE UPDATE ON evento_nfag
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_ret_nfag_updated_at BEFORE UPDATE ON ret_nfag
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add trigger to main nfag table
CREATE TRIGGER update_nfag_updated_at BEFORE UPDATE ON nfag
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
