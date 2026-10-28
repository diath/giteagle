import type { ThemeRegistration } from 'shiki/core'

import { color } from './palette'

export const SYNTAX_THEME_NAME = 'giteagle'

/** A Shiki theme built from the palette; Shiki's own themes are never used. */
export const syntaxTheme: ThemeRegistration = {
  name: SYNTAX_THEME_NAME,
  type: 'dark',
  colors: {
    'editor.background': color('bgSunken'),
    'editor.foreground': color('fg'),
  },
  tokenColors: [
    {
      scope: ['comment', 'punctuation.definition.comment'],
      settings: { foreground: color('syntaxComment'), fontStyle: 'italic' },
    },
    {
      scope: ['string', 'string.quoted', 'string.template', 'punctuation.definition.string'],
      settings: { foreground: color('syntaxString') },
    },
    {
      scope: ['string.regexp', 'constant.character.escape'],
      settings: { foreground: color('syntaxRegex') },
    },
    {
      scope: ['constant.numeric', 'constant.language', 'constant.character'],
      settings: { foreground: color('syntaxNumber') },
    },
    {
      scope: ['variable.other.constant', 'support.constant', 'constant.other'],
      settings: { foreground: color('syntaxConstant') },
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
      settings: { foreground: color('syntaxKeyword') },
    },
    {
      scope: ['keyword.operator', 'punctuation', 'meta.brace'],
      settings: { foreground: color('syntaxOperator') },
    },
    {
      scope: ['entity.name.function', 'support.function', 'meta.function-call.generic'],
      settings: { foreground: color('syntaxFunction') },
    },
    {
      scope: [
        'entity.name.type',
        'entity.name.class',
        'entity.other.inherited-class',
        'support.type',
        'support.class',
      ],
      settings: { foreground: color('syntaxType') },
    },
    {
      scope: ['entity.name.namespace', 'entity.name.module', 'entity.name.scope-resolution'],
      settings: { foreground: color('syntaxNamespace') },
    },
    {
      scope: ['variable', 'variable.other'],
      settings: { foreground: color('syntaxVariable') },
    },
    {
      scope: ['variable.parameter'],
      settings: { foreground: color('syntaxParameter') },
    },
    {
      scope: [
        'variable.other.property',
        'variable.other.object.property',
        'support.type.property-name',
        'meta.object-literal.key',
        'entity.name.tag.yaml',
      ],
      settings: { foreground: color('syntaxProperty') },
    },
    {
      scope: ['entity.name.tag', 'meta.tag'],
      settings: { foreground: color('syntaxTag') },
    },
    {
      scope: ['entity.other.attribute-name'],
      settings: { foreground: color('syntaxAttribute') },
    },
    {
      scope: ['markup.heading', 'entity.name.section'],
      settings: { foreground: color('syntaxHeading'), fontStyle: 'bold' },
    },
    { scope: ['markup.bold'], settings: { fontStyle: 'bold' } },
    { scope: ['markup.italic'], settings: { fontStyle: 'italic' } },
    { scope: ['markup.inserted'], settings: { foreground: color('diffAddedSign') } },
    { scope: ['markup.deleted'], settings: { foreground: color('diffRemovedSign') } },
  ],
}
