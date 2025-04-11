import express from 'express';
import cors from 'cors';
import { checkDatabaseConnection } from './database';
import { callProcedures } from './database/procedures';

const app = express();
const port = process.env.PORT || 3000;

app.use(cors());
app.use(express.json());

app.get('/api/organizations', async (req, res) => {
  try {
    const result = await callProcedures('get_organization_statistics');
    res.json(result);
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
});

app.get('/api/users', async (req, res) => {
  try {
    const result = await callProcedures('get_users_by_organization');
    res.json(result);
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
});

app.get('/api/permissions', async (req, res) => {
  try {
    const result = await callProcedures('get_users_by_organization');
    res.json(result);
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
});

app.listen(port, () => {
  console.log(`Server is running on port ${port}`);
});
