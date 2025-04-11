import { createProcedures } from './procedures';

export async function initializeDatabase() {
    try {
        await createProcedures();
        console.log('Database initialization completed successfully');
    } catch (error) {
        console.error('Error initializing database:', error);
        throw error;
    }
} 