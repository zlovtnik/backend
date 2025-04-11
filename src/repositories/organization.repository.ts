import { PrismaClient } from '@prisma/client';

const prisma = new PrismaClient();

import { Organization, CreateOrganizationInput, UpdateOrganizationInput } from '../types/organization'

export const organizationRepository = {
    async create(data: CreateOrganizationInput): Promise<Organization> {
        return prisma.organization.create({
            data,
        })
    },

    async findById(id: string): Promise<Organization | null> {
        return prisma.organization.findUnique({
            where: { id },
        })
    },

    async findAll(): Promise<Organization[]> {
        return prisma.organization.findMany()
    },

    async update(id: string, data: UpdateOrganizationInput): Promise<Organization> {
        return prisma.organization.update({
            where: { id },
            data,
        })
    },

    async delete(id: string): Promise<Organization> {
        return prisma.organization.delete({
            where: { id },
        })
    },
} 