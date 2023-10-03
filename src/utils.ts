

export const nameFormat = (name: string) => {
  if (name.length === 0) {
    return ''
  }
  return name[0].toUpperCase() + name.replace(/[-_]/g, ' ').slice(1)
}