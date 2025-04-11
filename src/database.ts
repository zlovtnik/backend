import { Pool } from 'pg';

const pool = new Pool({
    user: process.env.DB_USER,
    host: process.env.DB_HOST,
    database: process.env.DB_NAME,
    password: process.env.DB_PASSWORD,
    port: parseInt(process.env.DB_PORT || '5432'),
});

export const checkDatabaseConnection = async () => {
    try {
        const client = await pool.connect();
        console.log('Database connection successful');
        client.release();
        return true;
    } catch (error) {
        console.error('Database connection failed:', error);
        return false;
    }
};

const healthCheck = async () => {
    const isConnected = await checkDatabaseConnection();
    if (!isConnected) {
        console.error('Database health check failed');
    }
};

setInterval(healthCheck, 30000);

export default pool; 