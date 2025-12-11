-- Drop indexes
DROP INDEX IF EXISTS idx_ret_tenant_id;
DROP INDEX IF EXISTS idx_evento_chNFAg;
DROP INDEX IF EXISTS idx_evento_tenant_id;
DROP INDEX IF EXISTS idx_cons_stat_tenant_id;
DROP INDEX IF EXISTS idx_cons_sit_chNFAg;
DROP INDEX IF EXISTS idx_cons_sit_tenant_id;
DROP INDEX IF EXISTS idx_nfag_total_nfag_id;
DROP INDEX IF EXISTS idx_nfag_dest_nfag_id;
DROP INDEX IF EXISTS idx_nfag_emit_nfag_id;
DROP INDEX IF EXISTS idx_nfag_ide_nfag_id;
DROP INDEX IF EXISTS idx_nfag_status;
DROP INDEX IF EXISTS idx_nfag_chave;
DROP INDEX IF EXISTS idx_nfag_tenant_id;

-- Drop tables in reverse order of creation
DROP TABLE IF EXISTS ret_nfag;
DROP TABLE IF EXISTS evento_nfag;
DROP TABLE IF EXISTS cons_stat_serv_nfag;
DROP TABLE IF EXISTS cons_sit_nfag;
DROP TABLE IF EXISTS nfag_total;
DROP TABLE IF EXISTS nfag_dest;
DROP TABLE IF EXISTS nfag_emit;
DROP TABLE IF EXISTS nfag_ide;
DROP TABLE IF EXISTS nfag;
