export type User = {
    id: string
    email: string
    name: string
    password: string
    organizationId: string
    createdAt: Date
    updatedAt: Date
}

export type CreateUserInput = {
    email: string
    name: string
    password: string
    organizationId: string
}

export type UpdateUserInput = {
    email?: string
    name?: string
    password?: string
    organizationId?: string
} 