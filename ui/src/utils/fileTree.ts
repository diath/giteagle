// Changed files grouped into a tree of folders, for the file list next to a
// diff. Folders come first and names sort case-insensitively, as in the Files
// tab.

export interface FileTreeFolder {
  kind: 'folder'
  /** The full path, which keys the folder's collapsed state. */
  path: string
  /** A chain of folders that hold nothing but the next one shows as one, e.g. "src/components". */
  name: string
  children: FileTreeNode[]
}

export interface FileTreeFile {
  kind: 'file'
  /** The file's position in the list the tree was built from. */
  index: number
  name: string
}

export type FileTreeNode = FileTreeFolder | FileTreeFile

export interface FileTreeRow {
  node: FileTreeNode
  depth: number
}

interface FolderBuilder {
  name: string
  path: string
  folders: Map<string, FolderBuilder>
  files: FileTreeFile[]
}

function newFolder(name: string, path: string): FolderBuilder {
  return { name, path, folders: new Map(), files: [] }
}

// Plain code unit order rather than the locale's, as the Files tab sorts.
function compareText(left: string, right: string): number {
  if (left < right) {
    return -1
  }
  return left > right ? 1 : 0
}

function byName(left: { name: string }, right: { name: string }): number {
  return compareText(left.name.toLowerCase(), right.name.toLowerCase()) || compareText(left.name, right.name)
}

function finishFolder(builder: FolderBuilder): FileTreeFolder {
  let current = builder
  let name = builder.name
  while (current.files.length === 0 && current.folders.size === 1) {
    const [only] = current.folders.values()
    name = `${name}/${only.name}`
    current = only
  }
  return { kind: 'folder', path: current.path, name, children: finishChildren(current) }
}

function finishChildren(builder: FolderBuilder): FileTreeNode[] {
  const folders = [...builder.folders.values()].map(finishFolder).sort(byName)
  const files = [...builder.files].sort(byName)
  return [...folders, ...files]
}

/** The top level of the tree for `paths`, slash-separated as Git writes them. */
export function buildFileTree(paths: readonly string[]): FileTreeNode[] {
  const root = newFolder('', '')
  paths.forEach((path, index) => {
    const parts = path.split('/')
    const name = parts.pop() ?? path
    let folder = root
    for (const part of parts) {
      let child = folder.folders.get(part)
      if (!child) {
        child = newFolder(part, folder.path === '' ? part : `${folder.path}/${part}`)
        folder.folders.set(part, child)
      }
      folder = child
    }
    folder.files.push({ kind: 'file', index, name })
  })
  return finishChildren(root)
}

/** The rows to show, leaving out what collapsed folders hold. */
export function visibleRows(nodes: readonly FileTreeNode[], collapsed: ReadonlySet<string>): FileTreeRow[] {
  const rows: FileTreeRow[] = []
  const visit = (children: readonly FileTreeNode[], depth: number): void => {
    for (const node of children) {
      rows.push({ node, depth })
      if (node.kind === 'folder' && !collapsed.has(node.path)) {
        visit(node.children, depth + 1)
      }
    }
  }
  visit(nodes, 0)
  return rows
}

/** The files' positions in their list, in the order the tree shows them. */
export function treeOrder(nodes: readonly FileTreeNode[]): number[] {
  const order: number[] = []
  const visit = (children: readonly FileTreeNode[]): void => {
    for (const node of children) {
      if (node.kind === 'folder') {
        visit(node.children)
      } else {
        order.push(node.index)
      }
    }
  }
  visit(nodes)
  return order
}
