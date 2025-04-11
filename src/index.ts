import { Elysia } from 'elysia';
import { cors } from '@elysiajs/cors';
import { db } from './database/connection';
import { organizationRepository } from './repositories/organization.repository';
import { userRepository } from './repositories/user.repository';
import { permissionRepository } from './repositories/permission.repository';
import { checkDatabaseConnection } from './database/connection';

console.log('Starting server...');

const app = new Elysia()
  .use(cors({
    origin: ['http://localhost:4200'], // Allow requests from Angular dev server
    methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
    allowedHeaders: ['Content-Type', 'x-user-id', 'X-User-ID'], // Allow both lowercase and uppercase headers
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
  })
  .post('/api/organizations', async ({ body, headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        throw new Error('User ID is required');
      }

      console.log('Fetching user:', userId);
      const user = await userRepository.findById(userId);
      if (!user) {
        throw new Error('User not found');
      }

      console.log('Creating organization with data:', body);
      const organization = await organizationRepository.create(body as any);
      console.log('Organization created:', organization);
      return organization;
    } catch (error) {
      console.error('Error in POST /api/organizations:', error);
      throw error;
    }
  })
  .delete('/api/organizations/:id', async ({ params, headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        throw new Error('User ID is required');
      }

      console.log('Fetching user:', userId);
      const user = await userRepository.findById(userId);
      if (!user) {
        throw new Error('User not found');
      }

      // Check if there are any users associated with this organization
      const users = await userRepository.findByOrganization(params.id);
      if (users.length > 0) {
        throw new Error('Cannot delete organization: it has associated users. Please delete or reassign the users first.');
      }

      console.log('Deleting organization:', params.id);
      const organization = await organizationRepository.delete(params.id);
      if (!organization) {
        throw new Error('Organization not found');
      }
      console.log('Organization deleted:', organization);
      return organization;
    } catch (error) {
      console.error('Error in DELETE /api/organizations:', error);
      throw error;
    }
  })
  .get('/api/users', async ({ headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        throw new Error('User ID is required');
      }

      console.log('Fetching user:', userId);
      const user = await userRepository.findById(userId);
      if (!user) {
        throw new Error('User not found');
      }

      console.log('User found:', user);
      const users = await userRepository.findByOrganization(user.organizationId);
      console.log('Users found:', users);
      return users;
    } catch (error) {
      console.error('Error in GET /api/users:', error);
      throw error;
    }
  })
  .get('/api/permissions', async ({ headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        throw new Error('User ID is required');
      }

      console.log('Fetching user:', userId);
      const user = await userRepository.findById(userId);
      if (!user) {
        throw new Error('User not found');
      }

      console.log('User found:', user);
      const permissions = await permissionRepository.findByUser(userId);
      console.log('Permissions found:', permissions);
      return permissions;
    } catch (error) {
      console.error('Error in GET /api/permissions:', error);
      throw error;
    }
  });

const start = async () => {
  try {
    await checkDatabaseConnection();
    console.log('Database connection established');

    app.listen(3000);
    console.log('Server is running on http://localhost:3000');
  } catch (error) {
    console.error('Failed to start server:', error);
    process.exit(1);
  }
};

start();
