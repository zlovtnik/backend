import { PrismaClient } from '@prisma/client';

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