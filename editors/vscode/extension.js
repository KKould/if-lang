const vscode = require('vscode');
const cp = require('child_process');
const path = require('path');

const DIAGNOSTIC_COLLECTION = 'iflang';

let warnedMissingCli = false;
let pendingTimer = null;
let pendingDoc = null;

function activate(context) {
  const diagnostics = vscode.languages.createDiagnosticCollection(DIAGNOSTIC_COLLECTION);
  context.subscriptions.push(diagnostics);

  context.subscriptions.push(
    vscode.languages.registerDefinitionProvider('iflang', {
      provideDefinition: (document, position) => {
        const name = identifierAt(document, position);
        if (!name) {
          return null;
        }
        return findDefinitionLocations(name, document, position);
      }
    }),
    vscode.languages.registerReferenceProvider('iflang', {
      provideReferences: (document, position, context) => {
        const name = identifierAt(document, position);
        if (!name) {
          return [];
        }
        return findReferenceLocations(name, document, context.includeDeclaration);
      }
    })
  );

  const runDiagnostics = (document) => {
    if (!document || document.languageId !== 'iflang') {
      return;
    }

    const config = vscode.workspace.getConfiguration('iflang', document.uri);
    const mode = config.get('diagnosticsMode', 'onSave');
    if (mode === 'off') {
      diagnostics.delete(document.uri);
      return;
    }

    const cliPath = config.get('cliPath', 'if_lang');
    const source = document.getText();
    const cwd = workspaceCwd(document);

    runCheck(cliPath, source, cwd)
      .then((result) => {
        if (result.spawnError) {
          if (!warnedMissingCli) {
            warnedMissingCli = true;
            vscode.window.showWarningMessage(
              `IF Lang: unable to run '${cliPath}'. Diagnostics disabled until the CLI is available.`
            );
          }
          diagnostics.delete(document.uri);
          return;
        }

        const diagnostic = parseDiagnostic(result.stderr, document, source);
        if (diagnostic) {
          diagnostics.set(document.uri, [diagnostic]);
        } else {
          diagnostics.delete(document.uri);
        }
      })
      .catch(() => {
        diagnostics.delete(document.uri);
      });
  };

  const scheduleDiagnostics = (document) => {
    if (!document || document.languageId !== 'iflang') {
      return;
    }
    const config = vscode.workspace.getConfiguration('iflang', document.uri);
    const delay = config.get('diagnosticsDebounceMs', 250);
    if (pendingTimer) {
      clearTimeout(pendingTimer);
    }
    pendingDoc = document;
    pendingTimer = setTimeout(() => {
      const doc = pendingDoc;
      pendingDoc = null;
      pendingTimer = null;
      runDiagnostics(doc);
    }, delay);
  };

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      runDiagnostics(doc);
    }),
    vscode.workspace.onDidSaveTextDocument((doc) => {
      const config = vscode.workspace.getConfiguration('iflang', doc.uri);
      const mode = config.get('diagnosticsMode', 'onSave');
      if (mode === 'onSave') {
        runDiagnostics(doc);
      }
    }),
    vscode.workspace.onDidChangeTextDocument((event) => {
      const doc = event.document;
      const config = vscode.workspace.getConfiguration('iflang', doc.uri);
      const mode = config.get('diagnosticsMode', 'onSave');
      if (mode === 'onType') {
        scheduleDiagnostics(doc);
      }
    }),
    vscode.workspace.onDidCloseTextDocument((doc) => {
      diagnostics.delete(doc.uri);
    })
  );

  if (vscode.window.activeTextEditor) {
    runDiagnostics(vscode.window.activeTextEditor.document);
  }
}

function deactivate() {}

function workspaceCwd(document) {
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (folder && folder.uri && folder.uri.fsPath) {
    return folder.uri.fsPath;
  }
  if (document.fileName) {
    return path.dirname(document.fileName);
  }
  return process.cwd();
}

function runCheck(cliPath, source, cwd) {
  return new Promise((resolve) => {
    let stdout = '';
    let stderr = '';
    const child = cp.spawn(cliPath, ['check', '-'], { cwd });

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });
    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });
    child.on('error', (err) => {
      resolve({ code: 1, stdout, stderr, spawnError: err });
    });
    child.on('close', (code) => {
      resolve({ code, stdout, stderr, spawnError: null });
    });

    child.stdin.write(source);
    child.stdin.end();
  });
}

function parseDiagnostic(stderr, document, source) {
  if (!stderr) {
    return null;
  }
  const lines = stderr.trim().split(/\r?\n/);
  for (const line of lines) {
    const parseMatch = line.match(/^(parse|validation) error: (.+) at (\d+)$/);
    if (parseMatch) {
      const message = parseMatch[2];
      const byteOffset = Number(parseMatch[3]);
      const utf16Offset = byteOffsetToUtf16(source, byteOffset);
      const pos = document.positionAt(utf16Offset);
      let range = document.getWordRangeAtPosition(pos);
      if (!range) {
        range = new vscode.Range(pos, pos);
      }
      return new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Error);
    }

    const evalMatch = line.match(/^eval error: (.+)$/);
    if (evalMatch) {
      const message = evalMatch[1];
      const pos = new vscode.Position(0, 0);
      const range = new vscode.Range(pos, pos);
      return new vscode.Diagnostic(range, message, vscode.DiagnosticSeverity.Error);
    }
  }
  return null;
}

function byteOffsetToUtf16(text, byteOffset) {
  const buffer = Buffer.from(text, 'utf8');
  const safeOffset = Math.min(Math.max(byteOffset, 0), buffer.length);
  return buffer.slice(0, safeOffset).toString('utf8').length;
}

module.exports = {
  activate,
  deactivate
};

function identifierAt(document, position) {
  const range = document.getWordRangeAtPosition(position, /[A-Za-z_][A-Za-z0-9_]*/);
  if (!range) {
    return null;
  }
  return document.getText(range);
}

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function findDefinitionLocations(name, originDocument, position) {
  const docs = await openWorkspaceDocuments(originDocument);
  const originLocations = [];
  const otherLocations = [];
  for (const doc of docs) {
    const matches = scanDefinitions(doc, name);
    if (originDocument && doc.uri.toString() === originDocument.uri.toString()) {
      originLocations.push(...matches);
    } else {
      otherLocations.push(...matches);
    }
  }
  if (originLocations.length > 0 && originDocument && position) {
    return rankDefinitions(originLocations, originDocument, position);
  }
  return originLocations.concat(otherLocations);
}

async function findReferenceLocations(name, originDocument, includeDeclaration) {
  const docs = await openWorkspaceDocuments(originDocument);
  const locations = [];
  const pattern = new RegExp(`\\b${escapeRegExp(name)}\\b`, 'g');
  for (const doc of docs) {
    const text = doc.getText();
    const scanText = stripNonCode(text);
    const definitionOffsets = includeDeclaration ? new Set() : collectDefinitionOffsets(doc, name);
    let match;
    while ((match = pattern.exec(scanText)) !== null) {
      if (!includeDeclaration && definitionOffsets.has(match.index)) {
        continue;
      }
      const start = doc.positionAt(match.index);
      const end = doc.positionAt(match.index + name.length);
      locations.push(new vscode.Location(doc.uri, new vscode.Range(start, end)));
    }
  }
  return locations;
}

async function openWorkspaceDocuments(originDocument) {
  const uris = await vscode.workspace.findFiles('**/*.if', '**/{target,node_modules}/**');
  const docs = [];
  const originUri = originDocument && originDocument.uri ? originDocument.uri.toString() : null;
  let foundOrigin = false;
  for (const uri of uris) {
    if (originUri && uri.toString() === originUri) {
      docs.push(originDocument);
      foundOrigin = true;
      continue;
    }
    const doc = await vscode.workspace.openTextDocument(uri);
    docs.push(doc);
  }
  if (originDocument && !foundOrigin) {
    docs.unshift(originDocument);
  }
  return docs;
}

function scanDefinitions(document, name) {
  const text = document.getText();
  const scanText = stripNonCode(text);
  const locations = [];
  const patterns = [
    /\bextern\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g,
    /\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g,
    /\blet\s+([A-Za-z_][A-Za-z0-9_]*)\s*=/g,
    /\bdata\s+([A-Za-z_][A-Za-z0-9_]*)\s*=/g
  ];

  for (const pattern of patterns) {
    let match;
    while ((match = pattern.exec(scanText)) !== null) {
      if (match[1] !== name) {
        continue;
      }
      const startIndex = match.index + match[0].indexOf(match[1]);
      const start = document.positionAt(startIndex);
      const end = document.positionAt(startIndex + match[1].length);
      locations.push(new vscode.Location(document.uri, new vscode.Range(start, end)));
    }
  }

  if (/^[A-Z]/.test(name)) {
    locations.push(...scanDataVariants(document, scanText, name));
  }

  return locations;
}

function collectDefinitionOffsets(document, name) {
  const offsets = new Set();
  for (const location of scanDefinitions(document, name)) {
    offsets.add(document.offsetAt(location.range.start));
  }
  return offsets;
}

function scanDataVariants(document, scanText, name) {
  const locations = [];
  const dataPattern = /\bdata\s+([A-Za-z_][A-Za-z0-9_]*)\s*=/g;
  let dataMatch;
  while ((dataMatch = dataPattern.exec(scanText)) !== null) {
    const dataStart = dataMatch.index;
    const dataType = dataMatch[1];
    const bodyStart = dataMatch.index + dataMatch[0].length;
    const semiIndex = scanText.indexOf(';', bodyStart);
    const bodyEnd = semiIndex === -1 ? scanText.length : semiIndex;
    const body = scanText.slice(bodyStart, bodyEnd);
    const variantPattern = /\b[A-Z][A-Za-z0-9_]*\b/g;
    let variantMatch;
    while ((variantMatch = variantPattern.exec(body)) !== null) {
      const variantName = variantMatch[0];
      if (variantName !== name) {
        continue;
      }
      const absIndex = bodyStart + variantMatch.index;
      const start = document.positionAt(absIndex);
      const end = document.positionAt(absIndex + variantName.length);
      locations.push(new vscode.Location(document.uri, new vscode.Range(start, end)));
    }

    if (dataType === name) {
      const typeIndex = dataStart + dataMatch[0].indexOf(dataType);
      const start = document.positionAt(typeIndex);
      const end = document.positionAt(typeIndex + dataType.length);
      locations.push(new vscode.Location(document.uri, new vscode.Range(start, end)));
    }
  }
  return locations;
}

function rankDefinitions(locations, originDocument, position) {
  const originOffset = originDocument.offsetAt(position);
  const before = [];
  const after = [];
  for (const location of locations) {
    const offset = originDocument.offsetAt(location.range.start);
    if (offset <= originOffset) {
      before.push({ location, offset });
    } else {
      after.push({ location, offset });
    }
  }
  before.sort((a, b) => b.offset - a.offset);
  after.sort((a, b) => a.offset - b.offset);
  return before.concat(after).map((entry) => entry.location);
}

function stripNonCode(text) {
  let out = '';
  let i = 0;
  let state = 'code';
  while (i < text.length) {
    const ch = text[i];
    const next = i + 1 < text.length ? text[i + 1] : '';
    if (state === 'code') {
      if (ch === '/' && next === '/') {
        out += '  ';
        i += 2;
        state = 'linecomment';
        continue;
      }
      if (ch === 'b' && next === '\"') {
        out += '  ';
        i += 2;
        state = 'string';
        continue;
      }
      if (ch === '\"') {
        out += ' ';
        i += 1;
        state = 'string';
        continue;
      }
      out += ch;
      i += 1;
      continue;
    }
    if (state === 'linecomment') {
      if (ch === '\n') {
        out += '\n';
        i += 1;
        state = 'code';
        continue;
      }
      if (ch === '\r') {
        out += '\r';
        i += 1;
        continue;
      }
      out += ' ';
      i += 1;
      continue;
    }
    if (state === 'string') {
      if (ch === '\\\\') {
        if (i + 1 < text.length) {
          out += '  ';
          i += 2;
        } else {
          out += ' ';
          i += 1;
        }
        continue;
      }
      if (ch === '\"') {
        out += ' ';
        i += 1;
        state = 'code';
        continue;
      }
      out += ' ';
      i += 1;
      continue;
    }
  }
  return out;
}
