class RenameDownloadedPageToDownloadedSection < ActiveRecord::Migration[5.0]
  def change
    rename_table :downloaded_pages, :downloaded_sections
  end
end
