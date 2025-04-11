import { PrismaClient } from '@prisma/client';
import { Organization, CreateOrganizationInput, UpdateOrganizationInput } from '../types/organization'

const prisma = new PrismaClient();

export const organizationRepository = {
    async create(data: CreateOrganizationInput): Promise<Organization> {
        const result = await prisma.organization.create({
            data,
        });
        return {
            ...result,
            description: result.description || undefined
        };
    },

    async findById(id: string): Promise<Organization | null> {
        const result = await prisma.organization.findUnique({
            where: { id },
        });
        if (!result) return null;
        return {
            ...result,
            description: result.description || undefined
        };
    },

    async findAll(): Promise<Organization[]> {
        const results = await prisma.organization.findMany();
        return results.map((result: { description: string | null }) => ({
            ...result,
            description: result.description || undefined
        }));
    },

    async update(id: string, data: UpdateOrganizationInput): Promise<Organization> {
        const result = await prisma.organization.update({
            where: { id },
            data,
        });
        return {
            ...result,
            description: result.description || undefined
        };
    },

    async delete(id: string): Promise<Organization> {
        const result = await prisma.organization.delete({
            where: { id },
        });
        return {
            ...result,
            description: result.description || undefined
        };
    },
} 