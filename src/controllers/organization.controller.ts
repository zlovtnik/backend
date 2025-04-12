import { Request, Response } from 'express';
import { organizationRepository } from '../repositories/organization.repository';
import { CreateOrganizationInput, UpdateOrganizationInput } from '../types/organization';

export const organizationController = {
    async create(req: Request, res: Response) {
        try {
            const data: CreateOrganizationInput = req.body;
            const organization = await organizationRepository.create(data);
            res.status(201).json(organization);
        } catch (error) {
            console.error('Error creating organization:', error);
            res.status(500).json({ error: 'Failed to create organization' });
        }
    },

    async findById(req: Request, res: Response) {
        try {
            const { id } = req.params;
            const organization = await organizationRepository.findById(id);
            if (!organization) {
                return res.status(404).json({ error: 'Organization not found' });
            }
            res.json(organization);
        } catch (error) {
            console.error('Error finding organization:', error);
            res.status(500).json({ error: 'Failed to find organization' });
        }
    },

    async findAll(req: Request, res: Response) {
        try {
            const organizations = await organizationRepository.findAll();
            res.json(organizations);
        } catch (error) {
            console.error('Error finding organizations:', error);
            res.status(500).json({ error: 'Failed to find organizations' });
        }
    },

    async update(req: Request, res: Response) {
        try {
            const { id } = req.params;
            const data: UpdateOrganizationInput = req.body;
            const organization = await organizationRepository.update(id, data);
            res.json(organization);
        } catch (error) {
            console.error('Error updating organization:', error);
            res.status(500).json({ error: 'Failed to update organization' });
        }
    },

    async delete(req: Request, res: Response) {
        try {
            const { id } = req.params;
            const organization = await organizationRepository.delete(id);
            res.json(organization);
        } catch (error) {
            console.error('Error deleting organization:', error);
            res.status(500).json({ error: 'Failed to delete organization' });
        }
    },
}; 