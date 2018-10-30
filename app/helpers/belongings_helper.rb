module BelongingsHelper

  def gold_update_link(group_elem)
    if action_name == 'add_gold'
      belongings_add_gold_update_path( group_elem )
    else
      belongings_loose_gold_update_path( group_elem )
    end
  end

end
