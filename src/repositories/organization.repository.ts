import prisma from '../database/index'
import { Organization, CreateOrganizationInput, UpdateOrganizationInput } from '../types/organization'

export const organizationRepository = {
    async create(data: CreateOrganizationInput): Promise<Organization> {
        const result = await prisma.organization.create({
            data,
        });
        return {
            ...result,
            description: result.description || null
        };
    },

    async findById(id: string): Promise<Organization | null> {
        const result = await prisma.organization.findUnique({
            where: { id },
        });
        if (!result) return null;
        return {
            ...result,
            description: result.description || null
        };
    },

    async findAll(): Promise<Organization[]> {
        const results = await prisma.organization.findMany();
        return results.map((result: { id: string; name: string; description: string | null; createdAt: Date; updatedAt: Date }) => ({
            id: result.id,
            name: result.name,
            description: result.description || null,
            createdAt: result.createdAt,
            updatedAt: result.updatedAt
        }));
    },

    async update(id: string, data: UpdateOrganizationInput): Promise<Organization> {
        const result = await prisma.organization.update({
            where: { id },
            data,
        });
        return {
            ...result,
            description: result.description || null
        };
    },

    async delete(id: string): Promise<Organization> {
        const result = await prisma.organization.delete({
            where: { id },
        });
        return {
            ...result,
            description: result.description || null
        };
    },
} 