import type { ThemeRegistration } from 'shiki/core'

import { cssVar } from './palette'

export const SYNTAX_THEME_NAME = 'giteagle'

/**
 * A Shiki theme built from the palette; Shiki's own themes are never used.
 * Its colors are the palette's CSS variables, so highlighted code follows
 * the theme without being tokenized again.
 */
export const syntaxTheme: ThemeRegistration = {
  name: SYNTAX_THEME_NAME,
  colors: {
    'editor.background': cssVar('bgSunken'),
    'editor.foreground': cssVar('fg'),
  },
  tokenColors: [
    {
      scope: ['comment', 'punctuation.definition.comment'],
      settings: { foreground: cssVar('syntaxComment'), fontStyle: 'italic' },
    },
    {
      scope: ['string', 'string.quoted', 'string.template', 'punctuation.definition.string'],
      settings: { foreground: cssVar('syntaxString') },
    },
    {
      scope: ['string.regexp', 'constant.character.escape'],
      settings: { foreground: cssVar('syntaxRegex') },
    },
    {
      scope: ['constant.numeric', 'constant.language', 'constant.character'],
      settings: { foreground: cssVar('syntaxNumber') },
    },
    {
      scope: ['variable.other.constant', 'support.constant', 'constant.other'],
      settings: { foreground: cssVar('syntaxConstant') },
    },
    {
      scope: [
        'keyword',
        'keyword.control',
        'storage',
        'storage.type',
        'storage.modifier',
        'variable.language',
      ],
      settings: { foreground: cssVar('syntaxKeyword') },
    },
    {
      scope: ['keyword.operator', 'punctuation', 'meta.brace'],
      settings: { foreground: cssVar('syntaxOperator') },
    },
    {
      scope: ['entity.name.function', 'support.function', 'meta.function-call.generic'],
      settings: { foreground: cssVar('syntaxFunction') },
    },
    {
      scope: [
        'entity.name.type',
        'entity.name.class',
        'entity.other.inherited-class',
        'support.type',
        'support.class',
      ],
      settings: { foreground: cssVar('syntaxType') },
    },
    {
      scope: ['entity.name.namespace', 'entity.name.module', 'entity.name.scope-resolution'],
      settings: { foreground: cssVar('syntaxNamespace') },
    },
    {
      scope: ['variable', 'variable.other'],
      settings: { foreground: cssVar('syntaxVariable') },
    },
    {
      scope: ['variable.parameter'],
      settings: { foreground: cssVar('syntaxParameter') },
    },
    {
      scope: [
        'variable.other.property',
        'variable.other.object.property',
        'support.type.property-name',
        'meta.object-literal.key',
        'entity.name.tag.yaml',
      ],
      settings: { foreground: cssVar('syntaxProperty') },
    },
    {
      scope: ['entity.name.tag', 'meta.tag'],
      settings: { foreground: cssVar('syntaxTag') },
    },
    {
      scope: ['entity.other.attribute-name'],
      settings: { foreground: cssVar('syntaxAttribute') },
    },
    {
      scope: ['markup.heading', 'entity.name.section'],
      settings: { foreground: cssVar('syntaxHeading'), fontStyle: 'bold' },
    },
    { scope: ['markup.bold'], settings: { fontStyle: 'bold' } },
    { scope: ['markup.italic'], settings: { fontStyle: 'italic' } },
    { scope: ['markup.inserted'], settings: { foreground: cssVar('diffAddedSign') } },
    { scope: ['markup.deleted'], settings: { foreground: cssVar('diffRemovedSign') } },
  ],
}
