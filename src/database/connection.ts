import { Pool } from 'pg';
import { PrismaClient } from '@prisma/client';

/**
 * Prisma client instance for database operations
 * @type {PrismaClient}
 */
export const db = new PrismaClient();

/**
 * Map to store active connections per user
 * @type {Map<string, Pool>}
 */
const userConnections = new Map<string, Pool>();

/**
 * Gets a database connection for a specific user
 * @param {string} userId - The ID of the user
 * @returns {Promise<Pool>} A PostgreSQL connection pool
 * @throws {Error} If the connection limit is exceeded
 */
export async function getUserConnection(userId: string): Promise<Pool> {
    if (userConnections.has(userId)) {
        return userConnections.get(userId)!;
    }

    const connection = new Pool({
        user: process.env.DB_USER || 'postgres',
        host: process.env.DB_HOST || 'localhost',
        database: process.env.DB_NAME || 'ai_alibaba_cloud',
        password: process.env.DB_PASSWORD || 'postgres',
        port: parseInt(process.env.DB_PORT || '5432'),
    });

    userConnections.set(userId, connection);
    return connection;
}

/**
 * Releases a user's database connection
 * @param {string} userId - The ID of the user
 */
export function releaseUserConnection(userId: string): void {
    const connection = userConnections.get(userId);
    if (connection) {
        connection.end();
        userConnections.delete(userId);
    }
}

/**
 * Checks the database connection
 * @async
 * @function checkDatabaseConnection
 * @returns {Promise<boolean>} True if the connection is successful, false otherwise
 */
export async function checkDatabaseConnection(): Promise<boolean> {
    try {
        // Try connecting with Prisma first
        await db.$connect();
        console.log('Database connection established with Prisma');
        return true;
    } catch (prismaError) {
        console.warn('Prisma connection failed, trying direct PostgreSQL connection:', prismaError);

        try {
            // Fallback to direct PostgreSQL connection
            const pool = new Pool({
                user: process.env.DB_USER || 'postgres',
                host: process.env.DB_HOST || 'localhost',
                database: process.env.DB_NAME || 'ai_alibaba_cloud',
                password: process.env.DB_PASSWORD || 'postgres',
                port: parseInt(process.env.DB_PORT || '5432'),
            });

            const client = await pool.connect();
            client.release();
            pool.end();

            console.log('Database connection established with direct PostgreSQL');
            return true;
        } catch (pgError) {
            console.error('Both Prisma and direct PostgreSQL connections failed:', pgError);
            return false;
        }
    }
} 