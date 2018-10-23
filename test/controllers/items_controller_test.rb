require 'test_helper'

class ItemsControllerTest < ActionDispatch::IntegrationTest

  setup do
    @user = create(:user)
    sign_in @user

    @book = create( :book )
    @adventure = create( :adventure, book: @book, current_page: @book.first_page, user: @user, items: {} )
    @user.update!( current_adventure_id: @adventure.id )
  end

  test 'should update item' do
    patch items_url, params: { inventory: {} }
    assert_redirected_to items_url
  end

end
