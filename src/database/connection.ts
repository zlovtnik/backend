import { Pool, PoolClient } from 'pg';
import { PrismaClient } from '@prisma/client';

export const db = new PrismaClient();

// Map to store active connections per user
const userConnections = new Map<string, PoolClient[]>();

export async function getUserConnection(userId: string): Promise<PoolClient> {
    const user = await db.user.findUnique({
        where: { id: userId },
        select: { maxConnections: true }
    });

    if (!user) {
        throw new Error('User not found');
    }

    const activeConnections = userConnections.get(userId) || [];

    // Check if user has reached their connection limit
    if (activeConnections.length >= user.maxConnections) {
        throw new Error(`User has reached maximum connection limit of ${user.maxConnections}`);
    }

    // Create new connection
    const pool = new Pool({
        connectionString: process.env.DATABASE_URL,
        max: user.maxConnections
    });

    const client = await pool.connect();
    activeConnections.push(client);
    userConnections.set(userId, activeConnections);

    return client;
}

export async function releaseUserConnection(userId: string, client: PoolClient) {
    const activeConnections = userConnections.get(userId) || [];
    const index = activeConnections.indexOf(client);

    if (index !== -1) {
        activeConnections.splice(index, 1);
        userConnections.set(userId, activeConnections);
        await client.release();
    }
}

export async function checkDatabaseConnection() {
    try {
        await db.$connect();
        console.log('Database connection established');
        return true;
    } catch (error) {
        console.error('Database connection error:', error);
        return false;
    }
} 