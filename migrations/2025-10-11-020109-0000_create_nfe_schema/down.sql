-- Reverse migration for NFe 4.00 Schema
-- Drop tables in reverse order to handle foreign key constraints

-- Drop indexes first
DROP INDEX IF EXISTS idx_nfe_fiscal_info_document;
DROP INDEX IF EXISTS idx_nfe_references_document;
DROP INDEX IF EXISTS idx_nfe_payments_document;
DROP INDEX IF EXISTS idx_nfe_transport_document;
DROP INDEX IF EXISTS idx_nfe_cofins_item;
DROP INDEX IF EXISTS idx_nfe_pis_item;
DROP INDEX IF EXISTS idx_nfe_ipi_item;
DROP INDEX IF EXISTS idx_nfe_icms_item;
DROP INDEX IF EXISTS idx_nfe_items_product;
DROP INDEX IF EXISTS idx_nfe_items_document;
DROP INDEX IF EXISTS idx_nfe_products_codigo;
DROP INDEX IF EXISTS idx_nfe_products_tenant;
DROP INDEX IF EXISTS idx_nfe_documents_chave;
DROP INDEX IF EXISTS idx_nfe_documents_emissao;
DROP INDEX IF EXISTS idx_nfe_documents_tenant_status;

-- Drop partial unique indexes
DROP INDEX IF EXISTS idx_nfe_recipients_tenant_cpf;
DROP INDEX IF EXISTS idx_nfe_recipients_tenant_cnpj;

-- Drop tables (in reverse dependency order)
DROP TABLE IF EXISTS nfe_fiscal_info;
DROP TABLE IF EXISTS nfe_references;
DROP TABLE IF EXISTS nfe_payments;
DROP TABLE IF EXISTS nfe_transport_volumes;
DROP TABLE IF EXISTS nfe_transport;
DROP TABLE IF EXISTS nfe_cofins;
DROP TABLE IF EXISTS nfe_pis;
DROP TABLE IF EXISTS nfe_ipi;
DROP TABLE IF EXISTS nfe_icms;
DROP TABLE IF EXISTS nfe_items;
DROP TABLE IF EXISTS nfe_recipients;
DROP TABLE IF EXISTS nfe_emitters;
DROP TABLE IF EXISTS nfe_products;
DROP TABLE IF EXISTS nfe_documents;
