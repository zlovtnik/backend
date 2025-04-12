import { Elysia } from 'elysia';
import { cors } from '@elysiajs/cors';
import { organizationRepository } from './repositories/organization.repository';
import { userRepository } from './repositories/user.repository';
import { checkDatabaseConnection } from './database/connection';

/**
 * Main application server using Elysia
 * @class App
 * @description Handles all API routes and middleware configuration
 */
const app = new Elysia()
  .use(cors({
    origin: ['http://localhost:4200'],
    methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
    allowedHeaders: ['Content-Type', 'x-user-id', 'X-User-ID'],
    exposedHeaders: ['Content-Type'],
    credentials: true
  }))
  .onError(({ code, error, set }) => {
    console.error('Error details:', {
      code,
      error: error.message,
      stack: error.stack,
      timestamp: new Date().toISOString()
    });

    if (code === 'NOT_FOUND') {
      set.status = 404;
      return { error: 'Resource not found' };
    }

    set.status = 500;
    return { error: error.message || 'Internal server error' };
  })
  .onRequest(({ request }) => {
    console.log(`[${new Date().toISOString()}] ${request.method} ${request.url}`);
    console.log('Headers:', Object.fromEntries(request.headers.entries()));
  })
  .onResponse(({ request, set }) => {
    console.log(`[${new Date().toISOString()}] Response:`, {
      method: request.method,
      url: request.url,
      status: set.status
    });
  })
  .get('/api/health', () => ({ status: 'ok' }))
  .get('/health', () => ({ status: 'ok' }))
  .get('/api/organizations', async ({ headers, set }) => {
    try {
      const userId = headers['x-user-id'] || headers['X-User-ID'];
      if (!userId) {
        set.status = 400;
        return { error: 'User ID is required in x-user-id header' };
      }

      console.log('Fetching user:', userId);
      const user = await userRepository.findById(userId);
      if (!user) {
        set.status = 404;
        return { error: 'User not found' };
      }

      console.log('User found:', user);
      const organizations = await organizationRepository.findAll();
      console.log('Organizations found:', organizations);
      return organizations;
    } catch (error) {
      console.error('Error in GET /api/organizations:', error);
      throw error;
    }
  })
  .get('/organizations', async ({ headers, set }) => {
    try {
      const userId = headers['x-user-id'] || headers['X-User-ID'];
      if (!userId) {
        set.status = 400;
        return { error: 'User ID is required in x-user-id header' };
      }

      console.log('Fetching user:', userId);
      const user = await userRepository.findById(userId);
      if (!user) {
        set.status = 404;
        return { error: 'User not found' };
      }

      console.log('User found:', user);
      const organizations = await organizationRepository.findAll();
      console.log('Organizations found:', organizations);
      return organizations;
    } catch (error) {
      console.error('Error in GET /organizations:', error);
      throw error;
    }
  });

/**
 * Start the server
 * @async
 * @function startServer
 * @description Initializes the server and establishes database connection
 * @throws {Error} If database connection fails
 */
async function startServer() {
  try {
    const isConnected = await checkDatabaseConnection();
    if (!isConnected) {
      throw new Error('Failed to connect to database');
    }

    const port = process.env.PORT || 3000;
    app.listen(port);
    console.log(`Server is running on port ${port}`);
  } catch (error) {
    console.error('Failed to start server:', error);
    process.exit(1);
  }
}

startServer();
