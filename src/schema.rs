// @generated automatically by Diesel CLI.

diesel::table! {
    configuration (key) {
        #[max_length = 255]
        key -> Varchar,
        value -> Text,
        #[max_length = 100]
        category -> Nullable<Varchar>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    login_history (id) {
        id -> Int4,
        user_id -> Int4,
        login_timestamp -> Timestamptz,
    }
}

diesel::table! {
    nfe_cofins (id) {
        id -> Int4,
        nfe_item_id -> Int4,
        #[max_length = 3]
        cst -> Varchar,
        #[max_length = 1]
        modalidade_bc -> Nullable<Varchar>,
        valor_bc -> Nullable<Numeric>,
        aliquota_percentual -> Nullable<Numeric>,
        aliquota_valor -> Nullable<Numeric>,
        quantidade_vendida -> Nullable<Numeric>,
        valor -> Nullable<Numeric>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_documents (id) {
        id -> Int4,
        #[max_length = 36]
        tenant_id -> Varchar,
        #[max_length = 50]
        nfe_id -> Varchar,
        #[max_length = 3]
        serie -> Varchar,
        #[max_length = 9]
        numero -> Varchar,
        #[max_length = 2]
        modelo -> Varchar,
        #[max_length = 4]
        versao -> Varchar,
        #[max_length = 20]
        status -> Varchar,
        #[max_length = 1]
        tipo_operacao -> Varchar,
        #[max_length = 1]
        tipo_emissao -> Varchar,
        #[max_length = 1]
        finalidade -> Varchar,
        #[max_length = 1]
        indicador_presencial -> Varchar,
        data_emissao -> Timestamptz,
        data_saida_entrada -> Nullable<Timestamptz>,
        data_autorizacao -> Nullable<Timestamptz>,
        data_cancelamento -> Nullable<Timestamptz>,
        valor_total -> Numeric,
        valor_desconto -> Nullable<Numeric>,
        valor_frete -> Nullable<Numeric>,
        valor_seguro -> Nullable<Numeric>,
        valor_outras_despesas -> Nullable<Numeric>,
        valor_produtos -> Numeric,
        valor_impostos -> Numeric,
        #[max_length = 60]
        pedido_compra -> Nullable<Varchar>,
        #[max_length = 60]
        contrato -> Nullable<Varchar>,
        informacoes_adicionais -> Nullable<Text>,
        informacoes_fisco -> Nullable<Text>,
        #[max_length = 50]
        protocolo_autorizacao -> Nullable<Varchar>,
        motivo_cancelamento -> Nullable<Text>,
        justificativa_contingencia -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_emitters (id) {
        id -> Int4,
        #[max_length = 36]
        tenant_id -> Varchar,
        #[max_length = 14]
        cnpj -> Varchar,
        #[max_length = 11]
        cpf -> Nullable<Varchar>,
        #[max_length = 120]
        razao_social -> Varchar,
        #[max_length = 60]
        nome_fantasia -> Nullable<Varchar>,
        #[max_length = 14]
        inscricao_estadual -> Nullable<Varchar>,
        #[max_length = 14]
        inscricao_estadual_subst_tributario -> Nullable<Varchar>,
        #[max_length = 15]
        inscricao_municipal -> Nullable<Varchar>,
        #[max_length = 7]
        cnae -> Nullable<Varchar>,
        #[max_length = 1]
        regime_tributario -> Varchar,
        #[max_length = 125]
        logradouro -> Nullable<Varchar>,
        #[max_length = 10]
        numero -> Nullable<Varchar>,
        #[max_length = 60]
        complemento -> Nullable<Varchar>,
        #[max_length = 60]
        bairro -> Nullable<Varchar>,
        #[max_length = 7]
        codigo_municipio -> Nullable<Varchar>,
        #[max_length = 60]
        municipio -> Nullable<Varchar>,
        #[max_length = 2]
        uf -> Nullable<Varchar>,
        #[max_length = 8]
        cep -> Nullable<Varchar>,
        #[max_length = 4]
        codigo_pais -> Nullable<Varchar>,
        #[max_length = 60]
        pais -> Nullable<Varchar>,
        #[max_length = 14]
        telefone -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_fiscal_info (id) {
        id -> Int4,
        nfe_document_id -> Int4,
        #[max_length = 20]
        campo -> Varchar,
        texto -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_icms (id) {
        id -> Int4,
        nfe_item_id -> Int4,
        #[max_length = 3]
        cst -> Varchar,
        #[max_length = 1]
        modalidade_bc -> Nullable<Varchar>,
        valor_bc -> Nullable<Numeric>,
        aliquota -> Nullable<Numeric>,
        valor -> Nullable<Numeric>,
        #[max_length = 1]
        modalidade_bc_st -> Nullable<Varchar>,
        percentual_mva_st -> Nullable<Numeric>,
        percentual_reducao_bc_st -> Nullable<Numeric>,
        valor_bc_st -> Nullable<Numeric>,
        aliquota_st -> Nullable<Numeric>,
        valor_st -> Nullable<Numeric>,
        percentual_reducao_bc_efetiva -> Nullable<Numeric>,
        valor_bc_efetiva -> Nullable<Numeric>,
        aliquota_efetiva -> Nullable<Numeric>,
        valor_efetivo -> Nullable<Numeric>,
        #[max_length = 10]
        codigo_beneficio_fiscal -> Nullable<Varchar>,
        percentual_diferimento -> Nullable<Numeric>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_ipi (id) {
        id -> Int4,
        nfe_item_id -> Int4,
        #[max_length = 3]
        cst -> Varchar,
        #[max_length = 5]
        classe_enquadramento -> Nullable<Varchar>,
        #[max_length = 14]
        cnpj_produtor -> Nullable<Varchar>,
        #[max_length = 6]
        codigo_selo_controle -> Nullable<Varchar>,
        quantidade_selo -> Nullable<Int4>,
        #[max_length = 1]
        modalidade_bc -> Nullable<Varchar>,
        valor_bc -> Nullable<Numeric>,
        aliquota -> Nullable<Numeric>,
        quantidade_unidade -> Nullable<Numeric>,
        valor_unidade -> Nullable<Numeric>,
        valor -> Nullable<Numeric>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_items (id) {
        id -> Int4,
        nfe_document_id -> Int4,
        numero_item -> Int4,
        product_id -> Nullable<Int4>,
        #[max_length = 60]
        codigo -> Varchar,
        #[max_length = 14]
        ean -> Nullable<Varchar>,
        #[max_length = 120]
        descricao -> Varchar,
        #[max_length = 8]
        ncm -> Nullable<Varchar>,
        #[max_length = 4]
        cfop -> Varchar,
        #[max_length = 6]
        unidade -> Varchar,
        quantidade -> Numeric,
        valor_unitario -> Numeric,
        valor_total -> Numeric,
        valor_desconto -> Nullable<Numeric>,
        valor_frete -> Nullable<Numeric>,
        valor_seguro -> Nullable<Numeric>,
        valor_outras_despesas -> Nullable<Numeric>,
        valor_bc_icms -> Nullable<Numeric>,
        valor_icms -> Nullable<Numeric>,
        valor_bc_icms_st -> Nullable<Numeric>,
        valor_icms_st -> Nullable<Numeric>,
        valor_bc_ipi -> Nullable<Numeric>,
        valor_ipi -> Nullable<Numeric>,
        valor_bc_pis -> Nullable<Numeric>,
        valor_pis -> Nullable<Numeric>,
        valor_bc_cofins -> Nullable<Numeric>,
        valor_cofins -> Nullable<Numeric>,
        informacoes_adicionais -> Nullable<Text>,
        #[max_length = 15]
        numero_pedido_compra -> Nullable<Varchar>,
        #[max_length = 6]
        item_pedido_compra -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_payments (id) {
        id -> Int4,
        nfe_document_id -> Int4,
        #[max_length = 1]
        indicador_pagamento -> Varchar,
        #[max_length = 2]
        forma_pagamento -> Varchar,
        valor -> Numeric,
        #[max_length = 1]
        tipo_integracao -> Nullable<Varchar>,
        #[max_length = 14]
        cnpj_credenciadora -> Nullable<Varchar>,
        #[max_length = 20]
        bandeira -> Nullable<Varchar>,
        #[max_length = 20]
        numero_autorizacao -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_pis (id) {
        id -> Int4,
        nfe_item_id -> Int4,
        #[max_length = 3]
        cst -> Varchar,
        #[max_length = 1]
        modalidade_bc -> Nullable<Varchar>,
        valor_bc -> Nullable<Numeric>,
        aliquota_percentual -> Nullable<Numeric>,
        aliquota_valor -> Nullable<Numeric>,
        quantidade_vendida -> Nullable<Numeric>,
        valor -> Nullable<Numeric>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_products (id) {
        id -> Int4,
        #[max_length = 36]
        tenant_id -> Varchar,
        #[max_length = 60]
        codigo -> Varchar,
        #[max_length = 14]
        ean -> Nullable<Varchar>,
        #[max_length = 120]
        descricao -> Varchar,
        #[max_length = 8]
        ncm -> Nullable<Varchar>,
        #[max_length = 4]
        cfop -> Nullable<Varchar>,
        #[max_length = 6]
        unidade -> Varchar,
        valor_unitario -> Numeric,
        valor_frete -> Nullable<Numeric>,
        valor_seguro -> Nullable<Numeric>,
        valor_desconto -> Nullable<Numeric>,
        valor_outras_despesas -> Nullable<Numeric>,
        #[max_length = 3]
        icms_cst -> Nullable<Varchar>,
        icms_aliquota -> Nullable<Numeric>,
        #[max_length = 3]
        ipi_cst -> Nullable<Varchar>,
        ipi_aliquota -> Nullable<Numeric>,
        #[max_length = 3]
        pis_cst -> Nullable<Varchar>,
        pis_aliquota -> Nullable<Numeric>,
        #[max_length = 3]
        cofins_cst -> Nullable<Varchar>,
        cofins_aliquota -> Nullable<Numeric>,
        informacoes_adicionais -> Nullable<Text>,
        ativo -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_recipients (id) {
        id -> Int4,
        #[max_length = 36]
        tenant_id -> Varchar,
        #[max_length = 1]
        tipo_pessoa -> Varchar,
        #[max_length = 14]
        cnpj -> Nullable<Varchar>,
        #[max_length = 11]
        cpf -> Nullable<Varchar>,
        #[max_length = 20]
        id_estrangeiro -> Nullable<Varchar>,
        #[max_length = 120]
        razao_social -> Varchar,
        #[max_length = 60]
        nome_fantasia -> Nullable<Varchar>,
        #[max_length = 14]
        inscricao_estadual -> Nullable<Varchar>,
        #[max_length = 15]
        inscricao_municipal -> Nullable<Varchar>,
        #[max_length = 9]
        inscricao_suframa -> Nullable<Varchar>,
        #[max_length = 60]
        email -> Nullable<Varchar>,
        #[max_length = 125]
        logradouro -> Nullable<Varchar>,
        #[max_length = 10]
        numero -> Nullable<Varchar>,
        #[max_length = 60]
        complemento -> Nullable<Varchar>,
        #[max_length = 60]
        bairro -> Nullable<Varchar>,
        #[max_length = 7]
        codigo_municipio -> Nullable<Varchar>,
        #[max_length = 60]
        municipio -> Nullable<Varchar>,
        #[max_length = 2]
        uf -> Nullable<Varchar>,
        #[max_length = 8]
        cep -> Nullable<Varchar>,
        #[max_length = 4]
        codigo_pais -> Nullable<Varchar>,
        #[max_length = 60]
        pais -> Nullable<Varchar>,
        #[max_length = 14]
        telefone -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_references (id) {
        id -> Int4,
        nfe_document_id -> Int4,
        #[max_length = 1]
        tipo -> Varchar,
        #[max_length = 44]
        chave_acesso -> Nullable<Varchar>,
        #[max_length = 2]
        uf -> Nullable<Varchar>,
        #[max_length = 4]
        mes_ano -> Nullable<Varchar>,
        #[max_length = 14]
        cnpj -> Nullable<Varchar>,
        #[max_length = 2]
        modelo -> Nullable<Varchar>,
        #[max_length = 3]
        serie -> Nullable<Varchar>,
        #[max_length = 9]
        numero -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_transport (id) {
        id -> Int4,
        nfe_document_id -> Int4,
        #[max_length = 1]
        modalidade_frete -> Varchar,
        #[max_length = 14]
        cnpj -> Nullable<Varchar>,
        #[max_length = 11]
        cpf -> Nullable<Varchar>,
        #[max_length = 60]
        razao_social -> Nullable<Varchar>,
        #[max_length = 14]
        inscricao_estadual -> Nullable<Varchar>,
        #[max_length = 200]
        endereco_completo -> Nullable<Varchar>,
        #[max_length = 60]
        municipio -> Nullable<Varchar>,
        #[max_length = 2]
        uf -> Nullable<Varchar>,
        #[max_length = 8]
        placa_veiculo -> Nullable<Varchar>,
        #[max_length = 2]
        uf_veiculo -> Nullable<Varchar>,
        #[max_length = 20]
        rntc -> Nullable<Varchar>,
        valor_servico -> Nullable<Numeric>,
        valor_bc_retencao_icms -> Nullable<Numeric>,
        valor_icms_retido -> Nullable<Numeric>,
        #[max_length = 4]
        cfop -> Nullable<Varchar>,
        #[max_length = 7]
        codigo_municipio -> Nullable<Varchar>,
        informacoes_fisco -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    nfe_transport_volumes (id) {
        id -> Int4,
        nfe_transport_id -> Int4,
        quantidade -> Int4,
        #[max_length = 60]
        especie -> Nullable<Varchar>,
        #[max_length = 60]
        marca -> Nullable<Varchar>,
        #[max_length = 60]
        numeracao -> Nullable<Varchar>,
        peso_liquido -> Nullable<Numeric>,
        peso_bruto -> Nullable<Numeric>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    people (id) {
        id -> Int4,
        name -> Varchar,
        gender -> Bool,
        age -> Int4,
        address -> Varchar,
        #[max_length = 11]
        phone -> Varchar,
        email -> Varchar,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Varchar,
        expires_at -> Timestamptz,
        created_at -> Nullable<Timestamptz>,
        revoked -> Nullable<Bool>,
    }
}

diesel::table! {
    sessions (session_id) {
        #[max_length = 255]
        session_id -> Varchar,
        #[max_length = 255]
        user_id -> Varchar,
        created_at -> Nullable<Timestamptz>,
        expires_at -> Timestamptz,
        is_valid -> Nullable<Bool>,
    }
}

diesel::table! {
    tenants (id) {
        id -> Varchar,
        name -> Varchar,
        db_url -> Text,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        email -> Varchar,
        password -> Varchar,
        login_session -> Varchar,
        active -> Bool,
    }
}

diesel::joinable!(login_history -> users (user_id));
diesel::joinable!(nfe_cofins -> nfe_items (nfe_item_id));
diesel::joinable!(nfe_fiscal_info -> nfe_documents (nfe_document_id));
diesel::joinable!(nfe_icms -> nfe_items (nfe_item_id));
diesel::joinable!(nfe_ipi -> nfe_items (nfe_item_id));
diesel::joinable!(nfe_items -> nfe_documents (nfe_document_id));
diesel::joinable!(nfe_items -> nfe_products (product_id));
diesel::joinable!(nfe_payments -> nfe_documents (nfe_document_id));
diesel::joinable!(nfe_pis -> nfe_items (nfe_item_id));
diesel::joinable!(nfe_references -> nfe_documents (nfe_document_id));
diesel::joinable!(nfe_transport -> nfe_documents (nfe_document_id));
diesel::joinable!(nfe_transport_volumes -> nfe_transport (nfe_transport_id));
diesel::joinable!(refresh_tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    configuration,
    login_history,
    nfe_cofins,
    nfe_documents,
    nfe_emitters,
    nfe_fiscal_info,
    nfe_icms,
    nfe_ipi,
    nfe_items,
    nfe_payments,
    nfe_pis,
    nfe_products,
    nfe_recipients,
    nfe_references,
    nfe_transport,
    nfe_transport_volumes,
    people,
    refresh_tokens,
    sessions,
    tenants,
    users,
);
