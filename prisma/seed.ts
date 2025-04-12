import { PrismaClient } from '@prisma/client';
import { faker } from '@faker-js/faker';

const prisma = new PrismaClient();

/**
 * Generates sample data for organizations, users, and permissions.
 * @returns {Promise<void>}
 */
async function seed() {
    try {
        // Create 50 organizations
        const organizations = await Promise.all(
            Array.from({ length: 50 }, () =>
                prisma.organization.create({
                    data: {
                        name: faker.company.name(),
                        description: faker.company.catchPhrase(),
                    },
                })
            )
        );

        // Create 20 permissions per organization
        const permissions = await Promise.all(
            organizations.flatMap((org) =>
                Array.from({ length: 20 }, () =>
                    prisma.permission.create({
                        data: {
                            name: faker.helpers.arrayElement([
                                'read',
                                'write',
                                'delete',
                                'admin',
                                'view',
                                'edit',
                                'create',
                                'manage',
                                'approve',
                                'reject',
                            ]),
                            description: faker.lorem.sentence(),
                            organizationId: org.id,
                        },
                    })
                )
            )
        );

        // Create 20 users per organization
        const users = await Promise.all(
            organizations.flatMap((org) =>
                Array.from({ length: 20 }, () =>
                    prisma.user.create({
                        data: {
                            name: faker.person.fullName(),
                            email: faker.internet.email(),
                            password: faker.internet.password(),
                            organizationId: org.id,
                        },
                    })
                )
            )
        );

        // Assign random permissions to users
        await Promise.all(
            users.map((user) => {
                const userPermissions = faker.helpers.arrayElements(
                    permissions.filter((p) => p.organizationId === user.organizationId),
                    { min: 1, max: 5 }
                );
                return prisma.user.update({
                    where: { id: user.id },
                    data: {
                        permissions: {
                            connect: userPermissions.map((p) => ({ id: p.id })),
                        },
                    },
                });
            })
        );

        console.log('Seed completed successfully');
    } catch (error) {
        console.error('Error seeding database:', error);
        throw error;
    } finally {
        await prisma.$disconnect();
    }
}

seed();
