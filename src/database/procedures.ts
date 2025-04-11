import { PrismaClient } from '@prisma/client';
import { Pool } from 'pg';
import pool from '../database.js';
import { getUserConnection, releaseUserConnection } from './connection';

const prisma = new PrismaClient();

export async function createProcedures() {
  try {
    await prisma.$executeRaw`
      CREATE OR REPLACE FUNCTION get_users_with_permissions(org_id UUID)
      RETURNS TABLE (
        user_id UUID,
        user_name TEXT,
        user_email TEXT,
        permission_names TEXT[]
      ) AS $$
      BEGIN
        RETURN QUERY
        SELECT 
          u.id as user_id,
          u.name as user_name,
          u.email as user_email,
          ARRAY_AGG(p.name) as permission_names
        FROM "User" u
        LEFT JOIN "Permission" p ON p.id IN (
          SELECT permission_id 
          FROM "_PermissionToUser" 
          WHERE user_id = u.id
        )
        WHERE u.organization_id = org_id
        GROUP BY u.id, u.name, u.email;
      END;
      $$ LANGUAGE plpgsql;
    `;

    await prisma.$executeRaw`
      CREATE OR REPLACE FUNCTION get_organization_stats(org_id UUID)
      RETURNS TABLE (
        total_users BIGINT,
        total_permissions BIGINT,
        created_at TIMESTAMP
      ) AS $$
      BEGIN
        RETURN QUERY
        SELECT 
          COUNT(DISTINCT u.id) as total_users,
          COUNT(DISTINCT p.id) as total_permissions,
          o.created_at
        FROM "Organization" o
        LEFT JOIN "User" u ON u.organization_id = o.id
        LEFT JOIN "Permission" p ON p.organization_id = o.id
        WHERE o.id = org_id
        GROUP BY o.created_at;
      END;
      $$ LANGUAGE plpgsql;
    `;

    console.log('Database procedures created successfully');
  } catch (error) {
    console.error('Error creating procedures:', error);
    throw error;
  }
}

export async function getUsersWithPermissions(organizationId: string) {
  return prisma.$queryRaw`
    SELECT * FROM get_users_with_permissions(${organizationId}::uuid)
  `;
}

export async function getOrganizationStats(organizationId: string) {
  return prisma.$queryRaw`
    SELECT * FROM get_organization_stats(${organizationId}::uuid)
  `;
}

export const getUsersByOrganization = async (organizationId: string, userId: string) => {
  const client = await getUserConnection(userId);
  try {
    const query = `
      SELECT 
        u.id,
        u.name,
        u.email,
        p.name as permission_name,
        p.description as permission_description
      FROM users u
      JOIN user_permissions up ON u.id = up.user_id
      JOIN permissions p ON up.permission_id = p.id
      WHERE u.organization_id = $1
    `;
    const result = await client.query(query, [organizationId]);
    return result.rows;
  } finally {
    await releaseUserConnection(userId, client);
  }
};

export const getOrganizationStatistics = async (organizationId: string, userId: string) => {
  const client = await getUserConnection(userId);
  try {
    const query = `
      SELECT 
        COUNT(DISTINCT u.id) as total_users,
        COUNT(DISTINCT p.id) as total_permissions,
        COUNT(DISTINCT up.id) as total_user_permissions
      FROM organizations o
      LEFT JOIN users u ON o.id = u.organization_id
      LEFT JOIN user_permissions up ON u.id = up.user_id
      LEFT JOIN permissions p ON up.permission_id = p.id
      WHERE o.id = $1
    `;
    const result = await client.query(query, [organizationId]);
    return result.rows[0];
  } finally {
    await releaseUserConnection(userId, client);
  }
};

export const callProcedures = async (procedureName: string, userId: string, params: any[] = []) => {
  const client = await getUserConnection(userId);
  try {
    const result = await client.query(`SELECT * FROM ${procedureName}($1)`, params);
    return result.rows;
  } catch (error) {
    console.error(`Error calling procedure ${procedureName}:`, error);
    throw error;
  } finally {
    await releaseUserConnection(userId, client);
  }
}; 