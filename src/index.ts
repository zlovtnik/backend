import express, { Router, RequestHandler } from 'express';
import cors from 'cors';
import { getOrganizationStatistics } from './database/procedures';
import { organizationRepository } from './repositories/organization.repository';

type OrganizationStats = {
  total_users: number;
  total_permissions: number;
  created_at: Date;
};

const app = express();
const router = Router();
const port = process.env.PORT || 3000;

app.use(cors());
app.use(express.json());

// Organization routes
const getOrganizations: RequestHandler = async (req, res) => {
  try {
    const organizations = await organizationRepository.findAll();
    res.json(organizations);
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
  return;
};

const getOrganizationById: RequestHandler = async (req, res) => {
  try {
    const organization = await organizationRepository.findById(req.params.id);
    if (!organization) {
      res.status(404).json({ error: 'Organization not found' });
      return;
    }
    const stats = await getOrganizationStatistics(req.params.id) as OrganizationStats[];
    res.json({ ...organization, stats: stats[0] });
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
  return;
};

const createOrganization: RequestHandler = async (req, res) => {
  try {
    const organization = await organizationRepository.create(req.body);
    res.status(201).json(organization);
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
  return;
};

const updateOrganization: RequestHandler = async (req, res) => {
  try {
    const organization = await organizationRepository.update(req.params.id, req.body);
    res.json(organization);
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
  return;
};

const deleteOrganization: RequestHandler = async (req, res) => {
  try {
    await organizationRepository.delete(req.params.id);
    res.status(204).send();
  } catch (error) {
    res.status(500).json({ error: 'Internal server error' });
  }
  return;
};

router.get('/organizations', getOrganizations);
router.get('/organizations/:id', getOrganizationById);
router.post('/organizations', createOrganization);
router.put('/organizations/:id', updateOrganization);
router.delete('/organizations/:id', deleteOrganization);

app.use('/api', router);

app.listen(port, () => {
  console.log(`Server is running on port ${port}`);
});
