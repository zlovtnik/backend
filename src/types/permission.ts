export type Permission = {
    id: string
    name: string
    description?: string
    organizationId: string
    createdAt: Date
    updatedAt: Date
}

export type CreatePermissionInput = {
    name: string
    description?: string
    organizationId: string
}

export type UpdatePermissionInput = {
    name?: string
    description?: string
    organizationId?: string
} 