# NFAg Database Schema Design

## Overview

Based on the analysis of XSD schemas in `PL_NFAg_1.00d/`, the following entities have been identified:

1. **NFAg** - Main water invoice entity
2. **consSitNFAg** - Consultation request for NFAg status
3. **retConsSitNFAg** - Response to consultation request
4. **eventoNFAg** - Events related to NFAg (cancellation, etc.)
5. **retEventoNFAg** - Response to event submission
6. **retNFAg** - Response to NFAg submission
7. **procNFAg** - Processed NFAg
8. **procEventoNFAg** - Processed event

## Data Type Mappings

XSD to PostgreSQL mappings:

- `xs:string` → `VARCHAR(n)` or `TEXT`
- `xs:decimal` → `DECIMAL(p,s)`
- `xs:int` → `INTEGER`
- `xs:date` → `DATE`
- `xs:dateTime` → `TIMESTAMP`
- `xs:boolean` → `BOOLEAN`

## Multi-Tenancy

All tables include `tenant_id` (UUID, foreign key to `tenants` table) for multi-tenancy support.

## Audit Fields

All tables include:

- `created_at` TIMESTAMP NOT NULL DEFAULT NOW()
- `updated_at` TIMESTAMP NOT NULL DEFAULT NOW()
- `created_by` UUID (foreign key to users)
- `updated_by` UUID (foreign key to users)

## Entity-Relationship Diagram (Text Representation)

```text
tenants (existing)
├── tenant_id (PK)

nfag
├── id (PK, UUID)
├── tenant_id (FK → tenants)
├── ch_nfag (VARCHAR(44), UNIQUE) - Chave de acesso
├── version (VARCHAR(10))
├── xml_content (TEXT) - Full XML storage option
├── status (VARCHAR(20)) - pending, authorized, rejected, etc.
├── created_at, updated_at, created_by, updated_by

nfag_ide
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── c_uf (INTEGER)
├── tp_amb (INTEGER)
├── mod (INTEGER)
├── serie (INTEGER)
├── n_nf (BIGINT)
├── c_nf (VARCHAR(8))
├── c_dv (VARCHAR(1))
├── dh_emi (TIMESTAMP)
├── tp_emis (INTEGER)
├── n_site_autoriz (INTEGER)
├── c_mun_fg (INTEGER)
├── fin_nfag (INTEGER)
├── tp_fat (INTEGER)
├── ver_proc (VARCHAR(20))
├── dh_cont (TIMESTAMP, NULL)
├── x_just (TEXT, NULL)
├── created_at, updated_at, etc.

nfag_emit
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── cnpj (VARCHAR(14))
├── ie (VARCHAR(15), NULL)
├── x_nome (VARCHAR(60))
├── x_fant (VARCHAR(60), NULL)
├── ender_emit_id (FK → addresses)
├── created_at, updated_at, etc.

nfag_dest
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── x_nome (VARCHAR(60))
├── cnpj (VARCHAR(14), NULL)
├── cpf (VARCHAR(11), NULL)
├── id_outros (VARCHAR(20), NULL)
├── ie (VARCHAR(15), NULL)
├── im (VARCHAR(15), NULL)
├── c_nis (VARCHAR(15), NULL)
├── nb (VARCHAR(10), NULL)
├── x_nome_adicional (VARCHAR(60), NULL)
├── ender_dest_id (FK → addresses)
├── created_at, updated_at, etc.

nfag_ligacao
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── id_ligacao (VARCHAR(20))
├── id_cod_cliente (VARCHAR(20), NULL)
├── tp_ligacao (INTEGER)
├── lat_gps (DECIMAL(10,8))
├── long_gps (DECIMAL(11,8))
├── cod_roteiro_leitura (VARCHAR(100), NULL)
├── created_at, updated_at, etc.

nfag_g_med
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── n_med (INTEGER)
├── id_medidor (VARCHAR(20))
├── d_med_ant (DATE)
├── d_med_atu (DATE)
├── created_at, updated_at, etc.

nfag_det
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── n_item (INTEGER)
├── c_prod (VARCHAR(60))
├── x_prod (VARCHAR(120))
├── c_class (VARCHAR(7))
├── tp_categoria (INTEGER, NULL)
├── x_categoria (VARCHAR(200), NULL)
├── q_economias (VARCHAR(5), NULL)
├── u_med (INTEGER)
├── q_faturada (DECIMAL(13,4))
├── v_item (DECIMAL(13,4))
├── fator_poluicao (DECIMAL(13,4), NULL)
├── v_prod (DECIMAL(13,4))
├── ind_devolucao (VARCHAR(1), NULL)
├── inf_ad_prod (TEXT, NULL)
├── ch_nfag_ant (VARCHAR(44), NULL)
├── n_item_ant (INTEGER, NULL)
├── created_at, updated_at, etc.

nfag_imposto
├── id (PK, UUID)
├── nfag_det_id (FK → nfag_det)
├── ibs_cbs_id (FK → ibs_cbs)
├── pis_cst (VARCHAR(2), NULL)
├── pis_v_bc (DECIMAL(13,2), NULL)
├── pis_p_pis (DECIMAL(5,4), NULL)
├── pis_v_pis (DECIMAL(13,2), NULL)
├── cofins_cst (VARCHAR(2), NULL)
├── cofins_v_bc (DECIMAL(13,2), NULL)
├── cofins_p_cofins (DECIMAL(5,4), NULL)
├── cofins_v_cofins (DECIMAL(13,2), NULL)
├── ret_trib_v_ret_pis (DECIMAL(13,2), NULL)
├── ret_trib_v_ret_cofins (DECIMAL(13,2), NULL)
├── ret_trib_v_ret_csll (DECIMAL(13,2), NULL)
├── ret_trib_v_irrf (DECIMAL(13,2), NULL)
├── tfs_v_bc_tfs (DECIMAL(13,2), NULL)
├── tfs_p_tfs (DECIMAL(5,4), NULL)
├── tfs_v_tfs (DECIMAL(13,2), NULL)
├── tfu_v_bc_tfu (DECIMAL(13,2), NULL)
├── tfu_p_tfu (DECIMAL(5,4), NULL)
├── tfu_v_tfu (DECIMAL(13,2), NULL)
├── created_at, updated_at, etc.

nfag_total
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── v_prod (DECIMAL(13,2))
├── v_ret_trib_pis (DECIMAL(13,2))
├── v_ret_trib_cofins (DECIMAL(13,2))
├── v_ret_trib_csll (DECIMAL(13,2))
├── v_ret_trib_irrf (DECIMAL(13,2))
├── v_cofins (DECIMAL(13,2))
├── v_pis (DECIMAL(13,2))
├── v_tfs (DECIMAL(13,2))
├── v_tfu (DECIMAL(13,2))
├── v_nf (DECIMAL(13,2))
├── ibs_cbs_tot_id (FK → ibs_cbs_tot)
├── v_tot_dfe (DECIMAL(13,2))
├── created_at, updated_at, etc.

addresses
├── id (PK, UUID)
├── tenant_id (FK → tenants)
├── x_lgr (VARCHAR(60))
├── nro (VARCHAR(60))
├── x_cpl (VARCHAR(60), NULL)
├── x_bairro (VARCHAR(60))
├── c_mun (INTEGER)
├── x_mun (VARCHAR(60))
├── cep (VARCHAR(8))
├── uf (VARCHAR(2))
├── fone (VARCHAR(12), NULL)
├── email (VARCHAR(60), NULL)
├── created_at, updated_at, etc.

ibs_cbs
├── id (PK, UUID)
├── tenant_id (FK → tenants)
├── v_bc (DECIMAL(13,2))
├── p_ibs (DECIMAL(5,4))
├── v_ibs (DECIMAL(13,2))
├── p_cbs (DECIMAL(5,4))
├── v_cbs (DECIMAL(13,2))
├── created_at, updated_at, etc.

ibs_cbs_tot
├── id (PK, UUID)
├── tenant_id (FK → tenants)
├── v_bc_ibs_tot (DECIMAL(13,2))
├── v_ibs_tot (DECIMAL(13,2))
├── v_bc_cbs_tot (DECIMAL(13,2))
├── v_cbs_tot (DECIMAL(13,2))
├── created_at, updated_at, etc.

cons_sit_nfag
├── id (PK, UUID)
├── tenant_id (FK → tenants)
├── tp_amb (INTEGER)
├── x_serv (VARCHAR(20))
├── ch_nfag (VARCHAR(44))
├── version (VARCHAR(10))
├── xml_content (TEXT)
├── status (VARCHAR(20))
├── created_at, updated_at, created_by, updated_by

ret_cons_sit_nfag
├── id (PK, UUID)
├── cons_sit_nfag_id (FK → cons_sit_nfag)
├── tp_amb (INTEGER)
├── ver_aplic (VARCHAR(20))
├── c_stat (VARCHAR(3))
├── x_motivo (TEXT)
├── c_uf (INTEGER)
├── prot_nfag_content (TEXT, NULL)
├── proc_evento_nfag_content (TEXT, NULL)
├── created_at, updated_at, etc.

evento_nfag
├── id (PK, UUID)
├── tenant_id (FK → tenants)
├── c_orgao (INTEGER)
├── tp_amb (INTEGER)
├── cnpj (VARCHAR(14))
├── ch_nfag (VARCHAR(44))
├── dh_evento (TIMESTAMP)
├── tp_evento (VARCHAR(6))
├── n_seq_evento (INTEGER)
├── det_evento_xml (TEXT)
├── versao_evento (VARCHAR(10))
├── xml_content (TEXT)
├── status (VARCHAR(20))
├── created_at, updated_at, created_by, updated_by

ret_evento_nfag
├── id (PK, UUID)
├── evento_nfag_id (FK → evento_nfag)
├── tp_amb (INTEGER)
├── ver_aplic (VARCHAR(20))
├── c_stat (VARCHAR(3))
├── x_motivo (TEXT)
├── c_orgao (INTEGER)
├── ch_nfag (VARCHAR(44))
├── dh_reg_evento (TIMESTAMP)
├── n_prot (VARCHAR(20))
├── created_at, updated_at, etc.

ret_nfag
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── tp_amb (INTEGER)
├── c_uf (INTEGER)
├── ver_aplic (VARCHAR(20))
├── c_stat (VARCHAR(3))
├── x_motivo (TEXT)
├── prot_nfag_content (TEXT, NULL)
├── created_at, updated_at, etc.

proc_nfag
├── id (PK, UUID)
├── nfag_id (FK → nfag)
├── xml_content (TEXT)
├── created_at, updated_at, etc.

proc_evento_nfag
├── id (PK, UUID)
├── evento_nfag_id (FK → evento_nfag)
├── xml_content (TEXT)
├── created_at, updated_at, etc.
```

## Indexes

- Primary keys on all tables
- Unique on `nfag.ch_nfag`
- Foreign key indexes
- Indexes on `tenant_id` for all tables
- Indexes on commonly queried fields: `ch_nfag`, `cnpj`, `dh_emi`, etc.

## Notes

- **XML Storage**: Option to store full XML in `xml_content` fields for compliance and reprocessing (max 10,000 characters)
- **Relationships**: One-to-many from NFAg to details, one-to-one for ide/emit/dest/ligacao/total
- **Normalization**: Complex nested structures normalized into separate tables
- **Constraints**: Based on XSD restrictions (length, patterns, enumerations)
- **Extensions**: Ready for future schema versions
