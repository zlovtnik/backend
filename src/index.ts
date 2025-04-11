import { Elysia } from 'elysia';
import { cors } from '@elysiajs/cors';
import { checkDatabaseConnection } from './database/connection';
import { callProcedures } from './database/procedures';

const app = new Elysia()
  .use(cors())
  .get('/api/organizations', async ({ headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        return new Response(JSON.stringify({ error: 'User ID is required' }), {
          status: 400,
          headers: { 'Content-Type': 'application/json' }
        });
      }
      const result = await callProcedures('get_organization_statistics', userId);
      return result;
    } catch (error) {
      return new Response(JSON.stringify({ error: 'Internal server error' }), {
        status: 500,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  })
  .get('/api/users', async ({ headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        return new Response(JSON.stringify({ error: 'User ID is required' }), {
          status: 400,
          headers: { 'Content-Type': 'application/json' }
        });
      }
      const result = await callProcedures('get_users_by_organization', userId);
      return result;
    } catch (error) {
      return new Response(JSON.stringify({ error: 'Internal server error' }), {
        status: 500,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  })
  .get('/api/permissions', async ({ headers }) => {
    try {
      const userId = headers['x-user-id'];
      if (!userId) {
        return new Response(JSON.stringify({ error: 'User ID is required' }), {
          status: 400,
          headers: { 'Content-Type': 'application/json' }
        });
      }
      const result = await callProcedures('get_users_by_organization', userId);
      return result;
    } catch (error) {
      return new Response(JSON.stringify({ error: 'Internal server error' }), {
        status: 500,
        headers: { 'Content-Type': 'application/json' }
      });
    }
  });

const port = process.env.PORT || 3000;

// Check database connection before starting the server
checkDatabaseConnection()
  .then(() => {
    app.listen(port, () => {
      console.log(`Server is running on port ${port}`);
    });
  })
  .catch((error) => {
    console.error('Failed to connect to database:', error);
    process.exit(1);
  });
