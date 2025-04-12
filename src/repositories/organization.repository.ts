import prisma from '../database/index'
import { Organization, CreateOrganizationInput, UpdateOrganizationInput } from '../types/organization'

/**
 * Repository for managing organization data in the database
 * @namespace organizationRepository
 */
export const organizationRepository = {
    /**
     * Creates a new organization in the database
     * @param {CreateOrganizationInput} data - The organization data to create
     * @returns {Promise<Organization>} The created organization
     * @throws {Error} If the organization creation fails
     */
    async create(data: CreateOrganizationInput): Promise<Organization> {
        const result = await prisma.organization.create({
            data,
        });
        return {
            ...result,
            description: result.description || null
        };
    },

    /**
     * Finds an organization by its ID
     * @param {string} id - The ID of the organization to find
     * @returns {Promise<Organization | null>} The found organization or null if not found
     */
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

    /**
     * Retrieves all organizations from the database
     * @returns {Promise<Organization[]>} An array of all organizations
     */
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

    /**
     * Updates an existing organization
     * @param {string} id - The ID of the organization to update
     * @param {UpdateOrganizationInput} data - The data to update
     * @returns {Promise<Organization>} The updated organization
     * @throws {Error} If the organization update fails
     */
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

    /**
     * Deletes an organization from the database
     * @param {string} id - The ID of the organization to delete
     * @returns {Promise<Organization>} The deleted organization
     * @throws {Error} If the organization deletion fails
     */
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