import { Box, Heading, Text } from '@chakra-ui/react'

interface Props {
    label: string
    value: number
}

export default function MetricsCard({ label, value }: Props) {
    return (
        <Box p="4" shadow="md" borderWidth="1px" borderRadius="md">
            <Heading size="sm" mb="2">{label}</Heading>
            <Text fontSize="2xl" fontWeight="bold">{value.toLocaleString()}</Text>
        </Box>
    )
}