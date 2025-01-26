searchState.loadedDescShard("bevy_image_font", 0, "<code>bevy_image_font</code>\nKerning as a floating point value, use this when you want …\nAn image font as well as the mapping of characters to …\nA Bevy plugin for rendering image-based fonts.\nA system set containing all systems related to the …\nText rendered using an <code>ImageFont</code>.\nHow kerning between characters is specified.\nKerning as an integer value, use this when you want a …\nThe glyph used to render <code>c</code> is contained in the part of the …\nThe layout of the texture atlas, describing the …\nThis module provides functionality for rendering text as …\nZero constant spacing between character\nSets the <code>font</code> field of this struct.\nThe handle to the <code>ImageFont</code> used to render this text. The …\nSets the <code>font_height</code> field of this struct.\nIf set, overrides the height the font is rendered at. This …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreates <code>Self</code> using <code>default()</code>.\nCreates <code>Self</code> using <code>default()</code>.\nCreates <code>Self</code> using <code>default()</code>.\nThe <code>ImageSampler</code> to use during font image rendering. The …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCode for parsing an <code>ImageFont</code> off of an on-disk …\nThis module provides functionality for rendering text as …\nMarks any text where the underlying <code>ImageFont</code> asset has …\nSets the <code>text</code> field of this struct.\nThe string of text to be rendered. Each character in the …\nThe image that contains the font glyphs. Each glyph is a …\nDebugging data for visualizing an <code>ImageFontSpriteText</code> in a …\nText rendered using an <code>ImageFont</code> as individual sprites.\nRounds fractional values during scaling.\nDetermines how scaling is applied when calculating the …\nRetains precise fractional values during scaling.\nTruncates fractional values during scaling.\nSets the <code>anchor</code> field of this struct.\nThe alignment point of the text relative to its position. …\nSets the <code>color</code> field of this struct.\nThe color applied to the rendered text. This color affects …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreates <code>Self</code> using <code>default()</code>.\nCreates <code>Self</code> using <code>default()</code>.\nCreates <code>Self</code> using <code>default()</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nSets the <code>letter_spacing</code> field of this struct.\nDetermines a constant kerning between characters. The …\nRenders gizmos for debugging <code>ImageFontText</code> and its …\nSets the <code>scaling_mode</code> field of this struct.\nDetermines how scaling is applied to the glyph dimensions …\nSystem that renders each <code>ImageFontText</code> as child <code>Sprite</code> …\nInterprets the string as a “grid” and slices up the …\nA repeated character was found in an <code>Automatic</code> layout …\nA validation error occurred on the <code>ImageFontDescriptor</code>. …\nThe image path provided is empty.\nThe layout string used for automatic character placement …\nOn-disk representation of an <code>ImageFont</code>, optimized to make …\nErrors that can show up during validation.\nHuman-readable way to specify where the characters in an …\nErrors that can show up during validation.\nErrors that can show up during loading.\nLoader for <code>ImageFont</code>s.\nConfiguration settings for the <code>ImageFontLoader</code>.\nThe image height does not evenly divide the number of …\nThe image width does not evenly divide the character count …\nThe path provided for the font’s image was not loaded as …\nAn I/O error occurred while loading the image font. This …\nA validation error occurred on the <code>ImageFontLayout</code>. …\nFailed to load an asset directly. This is usually caused …\nFully specifies the bounds of each character. The most …\nManually specifies the top-left position of each …\nThe asset path has no parent directory.\nThe path provided for the font’s image was not loaded as …\nParsing the on-disk representation of the font failed. …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreates <code>Self</code> using <code>default()</code>.\nCreates <code>Self</code> using <code>default()</code>.\nGets the path to the image file containing the font glyphs.\nThe <code>ImageSampler</code> to use during font image rendering. …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nGets the layout description of the font.\nProcesses the asset in an asynchronous closure.\nCreates a new <code>ImageFontDescriptor</code> instance with the …\nA mapping from characters to their top-left positions …\nThe size of each character, specified as a uniform width …\nThe character that was repeated in the layout string.\nThe column in the layout string where the repeated …\nThe height of the image being validated.\nThe number of lines in the layout.\nThe number of characters per line in the layout.\nThe row in the layout string where the repeated character …\nThe width of the image being validated.\nFailed to copy a character from the source font image …\nThe image could not be converted to a <code>DynamicImage</code>. This …\nA component for displaying in-world text that has been …\nA component for displaying UI text that has been …\nErrors that can occur during the rendering of an <code>ImageFont</code>.\nThe <code>ImageFont</code> asset required for rendering was not loaded. …\nThe texture asset associated with the <code>ImageFont</code> was not …\nAn unspecified internal error occurred during rendering. …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCreates <code>Self</code> using <code>default()</code>.\nCreates <code>Self</code> using <code>default()</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nSystem that renders each <code>ImageFontText</code> into its <code>ImageNode</code>. …\nSystem that renders each <code>ImageFontText</code> into its <code>Sprite</code>. …")