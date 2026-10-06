import GLib from 'gi://GLib';
import Soup from 'gi://Soup?version=3.0';
import Secret from 'gi://Secret?version=1';
import { createCitationKey } from './bibtex.js';
import { _ } from './i18n.js';

const API = 'https://api.zotero.org';
const API_VERSION = '3';
const ITEM_TEMPLATES = new Map();
const SECRET_SCHEMA = new Secret.Schema('org.ovenbird.Ovenbird', Secret.SchemaFlags.NONE, {
    application: Secret.SchemaAttributeType.STRING,
});

const ZOTERO_TO_BIB = {
    journalArticle: 'article',
    book: 'book',
    bookSection: 'incollection',
    conferencePaper: 'inproceedings',
    thesis: 'phdthesis',
    report: 'techreport',
    webpage: 'online',
    blogPost: 'online',
    magazineArticle: 'article',
    newspaperArticle: 'article',
    preprint: 'article',
};

const BIB_TO_ZOTERO = {
    article: 'journalArticle',
    book: 'book',
    incollection: 'bookSection',
    inproceedings: 'conferencePaper',
    phdthesis: 'thesis',
    mastersthesis: 'thesis',
    techreport: 'report',
    online: 'webpage',
    misc: 'document',
};

export function storeZoteroApiKey(apiKey) {
    const stored = Secret.password_store_sync(SECRET_SCHEMA, Secret.COLLECTION_DEFAULT,
        _('Ovenbird Zotero API key'), apiKey, null, { application: 'ovenbird' });
    if (!stored) throw new Error(_('The password service did not accept the key.'));
}

export function loadZoteroApiKey() {
    return Secret.password_lookup_sync(SECRET_SCHEMA, null, { application: 'ovenbird' });
}

export class ZoteroClient {
    constructor(userId, apiKey) {
        this.userId = String(userId).trim();
        this.apiKey = apiKey;
        this.session = new Soup.Session({ user_agent: 'Ovenbird/0.1.0', timeout: 30 });
    }

    async _itemTemplate(itemType) {
        const cached = ITEM_TEMPLATES.get(itemType);
        if (cached && cached.expiresAt > Date.now()) return cached;
        const query = `itemType=${encodeURIComponent(itemType)}`;
        const [templateResponse, creatorsResponse] = await Promise.all([
            this._request('GET', `${API}/items/new?${query}`),
            this._request('GET', `${API}/itemTypeCreatorTypes?${query}`),
        ]);
        const schema = {
            template: templateResponse.data,
            creatorTypes: new Set((creatorsResponse.data || []).map(item => item.creatorType)),
            expiresAt: Date.now() + 60 * 60 * 1000,
        };
        ITEM_TEMPLATES.set(itemType, schema);
        return schema;
    }

    async _request(method, url, body = null, headers = {}) {
        const message = Soup.Message.new(method, url);
        message.request_headers.append('Zotero-API-Version', API_VERSION);
        message.request_headers.append('Zotero-API-Key', this.apiKey);
        for (const [name, value] of Object.entries(headers))
            message.request_headers.append(name, String(value));
        if (body !== null) {
            message.set_request_body_from_bytes('application/json', new GLib.Bytes(
                new TextEncoder().encode(JSON.stringify(body))));
        }

        return new Promise((resolve, reject) => {
            this.session.send_and_read_async(message, GLib.PRIORITY_DEFAULT, null, (session, result) => {
                try {
                    const bytes = session.send_and_read_finish(result);
                    const status = message.get_status();
                    const responseText = new TextDecoder().decode(bytes.get_data() || new Uint8Array());
                    if (status < 200 || status >= 300) {
                        reject(new Error(_('Zotero returned %s: %s')
                            .replace('%s', status).replace('%s', responseText || message.get_reason_phrase())));
                        return;
                    }
                    let data = null;
                    if (responseText) {
                        try { data = JSON.parse(responseText); }
                        catch { data = responseText; }
                    }
                    resolve({ data, status, headers: message.response_headers });
                } catch (error) {
                    reject(error);
                }
            });
        });
    }

    async fetchAllItems() {
        const items = [];
        let start = 0;
        let libraryVersion = null;
        while (true) {
            const uri = `${API}/users/${encodeURIComponent(this.userId)}/items/top?limit=100&start=${start}`;
            const response = await this._request('GET', uri);
            libraryVersion = response.headers.get_one('Last-Modified-Version') || libraryVersion;
            const page = response.data || [];
            items.push(...page.filter(item => !['attachment', 'note'].includes(item.data?.itemType)));
            if (page.length < 100) break;
            start += page.length;
        }
        return { items, libraryVersion };
    }

    async sync(library) {
        const statePath = GLib.build_filenamev([library.directory, 'zotero-sync.json']);
        const state = this._readState(statePath);
        const { items: remoteItems, libraryVersion: fetchedLibraryVersion } = await this.fetchAllItems();
        let libraryVersion = fetchedLibraryVersion;
        const existingByZoteroKey = new Map(library.entries
            .filter(entry => entry.fields.zotero_key)
            .map(entry => [entry.fields.zotero_key, entry]));
        const existingByDoi = new Map(library.entries
            .filter(entry => entry.fields.doi)
            .map(entry => [entry.fields.doi.toLowerCase(), entry]));
        const usedKeys = new Set(library.entries.map(entry => entry.key.toLowerCase()));
        const pendingUpdates = [];
        const pendingCreates = [];
        const conflicts = [];
        let imported = 0;
        let updatedLocally = 0;
        let matched = 0;

        for (const remote of remoteItems) {
            const remoteKey = remote.key;
            const remoteHash = checksum(JSON.stringify(remote.data));
            let local = existingByZoteroKey.get(remoteKey) ||
                (remote.data.DOI ? existingByDoi.get(remote.data.DOI.toLowerCase()) : null);

            if (!local) {
                const importedEntry = this._toBibtex(remote, usedKeys);
                library.entries.push(importedEntry);
                existingByZoteroKey.set(remoteKey, importedEntry);
                state.items[remoteKey] = this._snapshot(remote, importedEntry, remoteHash);
                imported++;
                continue;
            }

            matched++;
            if (local.fields.zotero_key !== remoteKey) {
                local.fields.zotero_key = remoteKey;
                local.fields.zotero_version = String(remote.version);
            }

            const previous = state.items[remoteKey];
            const localHash = this._localHash(local);
            if (!previous) {
                const schema = await this._itemTemplate(remote.data.itemType);
                if (this._remoteDiffers(local, remote, schema.creatorTypes))
                    pendingUpdates.push({ local, remote });
                else
                    state.items[remoteKey] = this._snapshot(remote, local, remoteHash);
                continue;
            }

            const localChanged = localHash !== previous.localHash;
            const remoteChanged = remoteHash !== previous.remoteHash || remote.version !== previous.version;
            if (localChanged && remoteChanged) {
                conflicts.push({ key: local.key, title: local.fields.title || local.key, remoteKey });
                continue;
            }

            if (remoteChanged && !localChanged) {
                const updated = this._toBibtex(remote, usedKeys, local.key);
                const index = library.entries.findIndex(entry => entry.key === local.key);
                library.entries[index] = updated;
                local = updated;
                updatedLocally++;
                state.items[remoteKey] = this._snapshot(remote, local, remoteHash);
            } else if (localChanged && !remoteChanged) {
                pendingUpdates.push({ local, remote });
            } else {
                state.items[remoteKey] = this._snapshot(remote, local, remoteHash);
            }
        }

        for (const local of library.entries) {
            if (local.fields.zotero_key) continue;
            const duplicate = local.fields.doi ? remoteItems.find(item =>
                item.data.DOI?.toLowerCase() === local.fields.doi.toLowerCase()) : null;
            if (duplicate) {
                local.fields.zotero_key = duplicate.key;
                local.fields.zotero_version = String(duplicate.version);
                state.items[duplicate.key] = this._snapshot(duplicate, local,
                    checksum(JSON.stringify(duplicate.data)));
                continue;
            }
            pendingCreates.push({ local });
        }

        for (const { local, remote } of pendingUpdates) {
            const schema = await this._itemTemplate(remote.data.itemType);
            const data = { ...remote.data, ...this._toZotero(local, remote.data, schema.creatorTypes) };
            data.key = remote.key;
            data.version = remote.version;
            const uri = `${API}/users/${encodeURIComponent(this.userId)}/items/${remote.key}`;
            const updatedResponse = await this._request('PUT', uri, data);
            libraryVersion = updatedResponse.headers.get_one('Last-Modified-Version') || libraryVersion;
            const refreshedResponse = await this._request('GET', uri);
            const refreshed = refreshedResponse.data;
            libraryVersion = refreshedResponse.headers.get_one('Last-Modified-Version') || libraryVersion;
            local.fields.zotero_version = String(refreshed.version);
            state.items[remote.key] = this._snapshot(refreshed, local, checksum(JSON.stringify(refreshed.data)));
        }

        let createdCount = 0;
        let failedCount = 0;
        for (let i = 0; i < pendingCreates.length; i += 50) {
            const batch = pendingCreates.slice(i, i + 50);
            if (!libraryVersion)
                throw new Error(_('Zotero did not report the current library version; no references were uploaded.'));
            const batchData = await Promise.all(batch.map(async ({ local }) => {
                const itemType = BIB_TO_ZOTERO[local.type] || 'document';
                const schema = await this._itemTemplate(itemType);
                return { ...schema.template,
                    ...this._toZotero(local, schema.template, schema.creatorTypes) };
            }));
            const response = await this._request('POST', `${API}/users/${encodeURIComponent(this.userId)}/items`,
                batchData, { 'If-Unmodified-Since-Version': libraryVersion });
            libraryVersion = response.headers.get_one('Last-Modified-Version') || libraryVersion;
            const successful = response.data?.successful || {};
            const failed = response.data?.failed || {};
            failedCount += Object.keys(failed).length;
            for (const [index, result] of Object.entries(successful)) {
                const local = batch[Number(index)]?.local;
                if (!local) continue;
                const resultKey = typeof result === 'string' ? result : result?.key;
                if (!resultKey) continue;
                local.fields.zotero_key = resultKey;
                library.save();
                createdCount++;
                try {
                    const created = result?.data ? result : (await this._request('GET',
                        `${API}/users/${encodeURIComponent(this.userId)}/items/${resultKey}`)).data;
                    local.fields.zotero_version = String(created.version);
                    state.items[created.key] = this._snapshot(created, local, checksum(JSON.stringify(created.data)));
                } catch (error) {
                    state.items[resultKey] = { version: null, remoteHash: null, localHash: this._localHash(local) };
                    failedCount++;
                    logError(error, 'Could not read an item created in Zotero');
                }
            }
        }

        library.save();
        GLib.file_set_contents(statePath, JSON.stringify(state, null, 2));
        return { imported, updatedLocally, matched, created: createdCount, failed: failedCount, conflicts };
    }

    _readState(path) {
        try {
            const [ok, bytes] = GLib.file_get_contents(path);
            if (ok) return JSON.parse(new TextDecoder().decode(bytes));
        } catch (error) {
            logError(error, 'Could not read Zotero sync state');
        }
        return { items: {} };
    }

    _snapshot(remote, local, remoteHash) {
        return {
            version: remote.version,
            remoteHash,
            localHash: this._localHash(local),
        };
    }

    _localHash(entry) {
        const fields = { ...entry.fields };
        delete fields.zotero_key;
        delete fields.zotero_version;
        return checksum(JSON.stringify({ type: entry.type, fields }));
    }

    _remoteDiffers(entry, remote, creatorTypes) {
        const desired = this._toZotero(entry, remote.data, creatorTypes);
        return Object.entries(desired).some(([name, value]) => {
            const current = remote.data[name];
            if (name === 'creators') {
                const signature = creators => (creators || []).map(creator => [creator.creatorType,
                    creator.name || `${creator.lastName || ''},${creator.firstName || ''}`].join(':')).sort().join('|');
                return signature(value) !== signature(current);
            }
            if (name === 'tags') {
                const tags = list => (list || []).map(tag => tag.tag).sort().join('|');
                return tags(value) !== tags(current);
            }
            if (value === '' && (current === undefined || current === null || current === '')) return false;
            if (Array.isArray(value) && value.length === 0 && (!current || current.length === 0)) return false;
            return JSON.stringify(value) !== JSON.stringify(current);
        });
    }

    _toBibtex(remote, usedKeys, preferredKey = null) {
        const data = remote.data || {};
        const creatorName = creator => creator.name ? `{${creator.name}}` :
            `${creator.lastName || ''}${creator.firstName ? `, ${creator.firstName}` : ''}`.trim();
        const namesFor = type => (data.creators || []).filter(creator => creator.creatorType === type)
            .map(creatorName).filter(Boolean).join(' and ');
        const type = ZOTERO_TO_BIB[data.itemType] || 'misc';
        const publication = data.publicationTitle || data.bookTitle || data.proceedingsTitle || data.websiteTitle || '';
        const fields = {
            title: data.title || '',
            author: namesFor('author'),
            editor: namesFor('editor'),
            translator: namesFor('translator'),
            date: data.date || '',
            year: data.date?.match(/\d{4}/)?.[0] || '',
            journal: ['incollection', 'inproceedings'].includes(type) ? '' : publication,
            booktitle: ['incollection', 'inproceedings'].includes(type) ? publication : '',
            publisher: data.publisher || '',
            institution: data.institution || '',
            school: data.university || '',
            edition: data.edition || '',
            type: data.thesisType || data.reportType || '',
            volume: data.volume || '',
            number: data.issue || '',
            pages: data.pages || '',
            doi: data.DOI || '',
            url: data.url || '',
            abstract: data.abstractNote || '',
            keywords: (data.tags || []).map(tag => tag.tag).join(', '),
            zotero_key: remote.key,
            zotero_version: String(remote.version),
        };
        const key = preferredKey || createCitationKey(fields, usedKeys);
        usedKeys.add(key.toLowerCase());
        return { type, key, fields, rawFields: {} };
    }

    _toZotero(entry, template = null, creatorTypes = null) {
        const fields = entry.fields || {};
        const itemType = BIB_TO_ZOTERO[entry.type] || 'document';
        const allowedCreators = creatorTypes?.size ? creatorTypes : null;
        const creators = template?.creators
            ?.filter(creator => !['author', 'editor', 'translator'].includes(creator.creatorType))
            .map(creator => ({ ...creator })) || [];
        for (const creatorType of ['author', 'editor', 'translator']) {
            if (allowedCreators && !allowedCreators.has(creatorType)) continue;
            for (const name of (fields[creatorType] || '').split(/\s+and\s+/i).filter(Boolean)) {
                if (/^\{.*\}$/.test(name.trim())) {
                    creators.push({ creatorType, name: name.trim().slice(1, -1) });
                    continue;
                }
                const comma = name.indexOf(',');
                if (comma >= 0) creators.push({ creatorType, lastName: name.slice(0, comma).trim(),
                    firstName: name.slice(comma + 1).trim() });
                else {
                    const parts = name.trim().split(/\s+/);
                    creators.push({ creatorType, firstName: parts.slice(0, -1).join(' '), lastName: parts.at(-1) || '' });
                }
            }
        }
        const data = {
            itemType,
            title: fields.title || '',
            creators,
            date: fields.date || fields.year || '',
            publisher: fields.publisher || '',
            volume: fields.volume || '',
            issue: fields.number || '',
            pages: fields.pages || '',
            DOI: fields.doi || '',
            url: fields.url || '',
            abstractNote: fields.abstract || '',
            tags: (fields.keywords || '').split(/[,;]/).map(tag => tag.trim()).filter(Boolean).map(tag => ({ tag })),
        };
        if (entry.type === 'article') data.publicationTitle = fields.journal || '';
        if (entry.type === 'incollection') data.bookTitle = fields.booktitle || '';
        if (entry.type === 'inproceedings') data.proceedingsTitle = fields.booktitle || '';
        if (entry.type === 'online') data.websiteTitle = fields.journal || fields.organization || '';
        if (entry.type === 'techreport') data.institution = fields.institution || fields.publisher || '';
        if (['phdthesis', 'mastersthesis'].includes(entry.type)) {
            data.university = fields.school || fields.institution || '';
            data.thesisType = fields.type || (entry.type === 'mastersthesis' ? "Master's thesis" : 'PhD thesis');
        }
        if (entry.type === 'book') data.edition = fields.edition || '';
        if (!template) return data;
        const validFields = new Set(Object.keys(template));
        return Object.fromEntries(Object.entries(data).filter(([name]) => validFields.has(name)));
    }
}

function checksum(value) {
    return GLib.compute_checksum_for_string(GLib.ChecksumType.SHA256, value, -1);
}
