import { PrismaClient } from '@prisma/client';
<<<<<<< HEAD

const prisma = new PrismaClient();

async function main() {
    // Create an organization
    const organization = await prisma.organization.create({
        data: {
            name: 'Default Organization',
            description: 'Default organization for testing',
        },
    });

    // Create a user
    const user = await prisma.user.create({
        data: {
            email: 'test@example.com',
            name: 'Test User',
            password: 'hashed_password', // In a real app, this should be properly hashed
            organizationId: organization.id,
            maxConnections: 1,
        },
    });

    // Create some permissions
    const permissions = await Promise.all([
        prisma.permission.create({
            data: {
                name: 'read_organizations',
                description: 'Can read organizations',
                organizationId: organization.id,
            },
        }),
        prisma.permission.create({
            data: {
                name: 'write_organizations',
                description: 'Can write organizations',
                organizationId: organization.id,
            },
        }),
    ]);

    // Assign permissions to user
    await prisma.user.update({
        where: { id: user.id },
        data: {
            permissions: {
                connect: permissions.map(p => ({ id: p.id })),
            },
        },
    });

    console.log('Database has been seeded. 🌱');
    console.log('Test user ID:', user.id);
}

main()
    .catch((e) => {
        console.error(e);
        process.exit(1);
    })
    .finally(async () => {
        await prisma.$disconnect();
    }); 
=======
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
>>>>>>> 2d11823 (asd)
