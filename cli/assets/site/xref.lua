local function is_boundary(character)
  return character == nil or not character:match("[%w_-]")
end

local function replace_mentions(text, site_index, current_id)
  local result = {}
  local position = 1
  local changed = false

  while true do
    local start_position, end_position, identifier =
      text:find("([A-Z][A-Z][A-Z][A-Z]?%-[0-9][0-9][0-9][0-9])", position)
    if start_position == nil then
      break
    end

    if is_boundary(text:sub(start_position - 1, start_position - 1))
      and is_boundary(text:sub(end_position + 1, end_position + 1))
      -- Self references stay plain so a record does not link to its own page.
      and identifier ~= current_id
      and site_index[identifier] ~= nil then
      if start_position > position then
        table.insert(result, pandoc.Str(text:sub(position, start_position - 1)))
      end
      table.insert(
        result,
        pandoc.Link({ pandoc.Str(identifier) }, pandoc.utils.stringify(site_index[identifier]))
      )
      position = end_position + 1
      changed = true
    else
      table.insert(result, pandoc.Str(text:sub(position, end_position)))
      position = end_position + 1
    end
  end

  if not changed then
    return pandoc.Str(text)
  end
  if position <= #text then
    table.insert(result, pandoc.Str(text:sub(position)))
  end
  return result
end

function Pandoc(document)
  local site_index = document.meta.site_index
  if site_index == nil then
    return document
  end

  local current_id = pandoc.utils.stringify(document.meta.id)
  return document:walk({
    Str = function(element)
      return replace_mentions(element.text, site_index, current_id)
    end,
  })
end
