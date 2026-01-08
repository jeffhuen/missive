// Missive Mailbox Preview JavaScript

// ============================================================================
// Theme Management
// ============================================================================

function initTheme() {
    // Check for saved preference or system preference
    const saved = localStorage.getItem('missive-theme');
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

    if (saved === 'dark' || (!saved && prefersDark)) {
        document.documentElement.classList.add('dark');
    }
    updateThemeIcon();
}

function toggleTheme() {
    const isDark = document.documentElement.classList.toggle('dark');
    localStorage.setItem('missive-theme', isDark ? 'dark' : 'light');
    updateThemeIcon();
}

function updateThemeIcon() {
    const isDark = document.documentElement.classList.contains('dark');
    const sunIcon = document.getElementById('sun-icon');
    const moonIcon = document.getElementById('moon-icon');

    if (sunIcon && moonIcon) {
        sunIcon.style.display = isDark ? 'block' : 'none';
        moonIcon.style.display = isDark ? 'none' : 'block';
    }
}

// ============================================================================
// Date Formatting
// ============================================================================

function formatDate(isoString) {
    if (!isoString) return '';
    const date = new Date(isoString);
    return date.toLocaleString();
}

function formatRelativeDate(isoString) {
    if (!isoString) return '';
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString();
}

// ============================================================================
// State
// ============================================================================

let currentEmailId = null;
const basePath = window.location.pathname.replace(/\/?$/, '');

// ============================================================================
// Email Selection
// ============================================================================

async function selectEmail(id) {
    // Update selection UI
    document.querySelectorAll('.email-item').forEach(item => {
        item.classList.toggle('selected', item.dataset.id === id);
    });

    currentEmailId = id;

    try {
        const response = await fetch(`${basePath}/${id}`);
        if (!response.ok) throw new Error('Failed to load email');

        const email = await response.json();
        renderEmail(email);
    } catch (error) {
        console.error('Error loading email:', error);
        document.getElementById('email-view').innerHTML = `
            <div class="no-selection">
                <p>Error loading email</p>
            </div>
        `;
    }
}

// ============================================================================
// Email Rendering
// ============================================================================

function renderEmail(email) {
    const hasHtml = email.html_body != null;
    const hasText = email.text_body != null;

    // Build metadata section
    const metadataHtml = renderMetadata(email);

    // Build headers section (if any)
    const headersHtml = renderExtraMetadata('Headers', email.headers);

    // Build provider options section (if any)
    const providerOptionsHtml = renderProviderOptions(email.provider_options);

    // Build text body section (collapsible, expanded only if no HTML)
    const textBodyHtml = hasText ? renderTextBody(email.text_body, !hasHtml) : '';

    // Build HTML body section (always visible, not collapsible)
    const htmlBodyHtml = hasHtml ? renderHtmlBody(email.id) : '';

    // Build attachments section
    const attachmentsHtml = renderAttachments(email);

    document.getElementById('email-view').innerHTML = `
        <div class="email-detail">
            ${metadataHtml}
            ${headersHtml}
            ${providerOptionsHtml}
            ${textBodyHtml}
            ${htmlBodyHtml}
            ${attachmentsHtml}
        </div>
    `;
}

function renderMetadata(email) {
    const rows = [
        { label: 'From', value: email.from },
        { label: 'To', value: email.to?.join(', ') },
        { label: 'Subject', value: email.subject, fallback: 'No subject' },
        { label: 'Cc', value: email.cc?.join(', ') },
        { label: 'Bcc', value: email.bcc?.join(', ') },
        { label: 'Reply-To', value: email.reply_to },
        { label: 'Sent at', value: formatDate(email.sent_at) },
    ];

    const rowsHtml = rows
        .filter(row => row.value || row.fallback)
        .map(row => {
            const isEmpty = !row.value;
            const displayValue = row.value || row.fallback || 'n/a';
            return `
                <div class="metadata-row">
                    <dt class="metadata-label">${row.label}</dt>
                    <dd class="metadata-value${isEmpty ? ' empty' : ''}">${escapeHtml(displayValue)}</dd>
                </div>
            `;
        })
        .join('');

    return `<dl class="email-metadata">${rowsHtml}</dl>`;
}

function renderExtraMetadata(title, data) {
    if (!data || (typeof data === 'object' && Object.keys(data).length === 0)) {
        return '';
    }

    let items = '';
    if (typeof data === 'object' && !Array.isArray(data)) {
        items = Object.entries(data)
            .map(([key, value]) => `
                <div class="extra-metadata-item">
                    <span class="extra-metadata-key">${escapeHtml(key)}:</span>
                    <span class="extra-metadata-value">${escapeHtml(String(value))}</span>
                </div>
            `)
            .join('');
    }

    if (!items) return '';

    return `
        <div class="extra-metadata">
            <div class="extra-metadata-title">${title}</div>
            <div class="extra-metadata-grid">${items}</div>
        </div>
    `;
}

function renderProviderOptions(options) {
    if (!options || options.length === 0) return '';

    const items = options
        .map(opt => `
            <div class="extra-metadata-item">
                <span class="extra-metadata-key">${escapeHtml(opt.key)}:</span>
                <span class="extra-metadata-value">${escapeHtml(opt.value)}</span>
            </div>
        `)
        .join('');

    return `
        <div class="extra-metadata">
            <div class="extra-metadata-title">Provider Options</div>
            <div class="extra-metadata-grid">${items}</div>
        </div>
    `;
}

function renderTextBody(textBody, expanded = false) {
    const expandedClass = expanded ? ' expanded' : '';
    const chevronSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>`;

    return `
        <div class="collapsible-header${expandedClass}" onclick="toggleCollapsible(this)">
            ${chevronSvg}
            <span>Text body</span>
        </div>
        <div class="collapsible-content${expandedClass}">
            <div class="text-body-content">${escapeHtml(textBody)}</div>
        </div>
    `;
}

function renderHtmlBody(emailId) {
    const externalSvg = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path><polyline points="15 3 21 3 21 9"></polyline><line x1="10" y1="14" x2="21" y2="3"></line></svg>`;

    return `
        <div class="html-body-section">
            <div class="section-header">
                <span>HTML body</span>
                <a href="${basePath}/${emailId}/html" target="_blank" class="open-external" title="Open in new tab">
                    ${externalSvg}
                </a>
            </div>
            <div class="html-body-content">
                <iframe src="${basePath}/${emailId}/html" sandbox="allow-same-origin"></iframe>
            </div>
        </div>
    `;
}

function renderAttachments(email) {
    if (!email.attachments || email.attachments.length === 0) return '';

    const paperclipSvg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path></svg>`;

    const attachmentsHtml = email.attachments
        .map(att => `
            <a href="${basePath}/${email.id}/attachments/${att.index}"
               class="attachment-card"
               download="${escapeHtml(att.filename)}"
               target="_blank">
                ${paperclipSvg}
                <div>
                    <div class="attachment-name">${escapeHtml(att.filename)}</div>
                    <div class="attachment-meta">${escapeHtml(att.content_type)} - ${formatBytes(att.size)}</div>
                </div>
            </a>
        `)
        .join('');

    return `
        <div class="attachments-section">
            <div class="attachments-title">Attachments (${email.attachments.length})</div>
            <div class="attachments-grid">${attachmentsHtml}</div>
        </div>
    `;
}

// ============================================================================
// Collapsible Sections
// ============================================================================

function toggleCollapsible(header) {
    header.classList.toggle('expanded');
    const content = header.nextElementSibling;
    if (content) {
        content.classList.toggle('expanded');
    }
}

// ============================================================================
// Actions
// ============================================================================

async function clearAll() {
    if (!confirm('Clear all emails?')) return;

    try {
        await fetch(`${basePath}/clear`, { method: 'POST' });
        location.reload();
    } catch (error) {
        console.error('Error clearing emails:', error);
    }
}

// ============================================================================
// Utilities
// ============================================================================

function escapeHtml(str) {
    if (str == null) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#x27;');
}

function formatBytes(bytes) {
    if (bytes === 0 || bytes == null) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// ============================================================================
// Initialization
// ============================================================================

// Initialize theme
initTheme();

// Auto-select first email if available
const firstItem = document.querySelector('.email-item');
if (firstItem) {
    selectEmail(firstItem.dataset.id);
}
