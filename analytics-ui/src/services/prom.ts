import axios from 'axios'

export const PROM = axios.create({
    baseURL: '/api/v1',
})

export async function queryInstant(expr: string): Promise<number> {
    const { data } = await PROM.get('/query', { params: { query: expr } })
    const v = data.data.result[0]?.value
    return v ? parseFloat(v[1]) : 0
}

export async function queryRange(
    expr: string,
    start: number,
    end: number,
    step: string
): Promise<[number,string][]> {
    const { data } = await PROM.get('/query_range', {
        params: { query: expr, start, end, step }
    })
    const series = data.data.result[0]
    return series?.values as [number,string][] || []
}
